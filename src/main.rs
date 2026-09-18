use std::{path::PathBuf, process};

use git_release::{
    ecosystem::{
        cli,
        recipes::{cargo::CargoRecipe, npm::NpmRecipe},
        types::{Ecosystem, EcosystemType, ReleaseKind},
    },
    errors::AppError,
};
use git2::{Cred, CredentialType, PushOptions, RemoteCallbacks, Repository};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let matches = cli::cli();

    let release_type = matches
        .get_one::<ReleaseKind>("kind")
        .expect("required argument; validated by clap");

    let directory = matches
        .get_one::<PathBuf>("repo")
        .expect("defaulted by clap");

    let repo = Repository::open(directory)
        .map_err(|_| AppError::RepoNotFound(directory.display().to_string()))?;

    let should_push = matches.get_flag("push");

    let Some(ecosystem) = EcosystemType::detect(directory) else {
        eprintln!(
            "error: no supported manifest found in '{}'",
            directory.display()
        );
        eprintln!("       git-release supports Cargo.toml (Rust) and package.json (Node.js)");
        process::exit(1);
    };

    let (next_version, files) = match ecosystem {
        EcosystemType::Cargo => {
            CargoRecipe::new(directory.to_path_buf(), release_type).bump_package_version()
        }
        EcosystemType::Npm => {
            NpmRecipe::new(directory.to_path_buf(), release_type).bump_package_version()
        }
    }?;

    commit_changes(&repo, (&next_version, files))?;
    add_tag(&repo, &next_version)?;

    if should_push {
        push_release(&repo, &next_version)?;
        println!("Released v{next_version} and pushed.");
    } else {
        println!("Released v{next_version}. Run `git push --follow-tags` to publish.");
    }

    Ok(())
}

fn push_release(repo: &Repository, version: &str) -> Result<(), AppError> {
    let head = repo.head().map_err(AppError::Git)?;
    if !head.is_branch() {
        return Err(AppError::PushFailed(
            "HEAD is detached; check out a branch before using --push".to_owned(),
        ));
    }

    let local_ref = head.name().map_err(AppError::Git)?;
    let branch_name = head.shorthand().unwrap_or(local_ref);

    let remote_name = repo
        .branch_upstream_remote(local_ref)
        .map_err(|_| {
            AppError::PushFailed(format!("branch '{branch_name}' has no upstream remote"))
        })?
        .as_str()
        .map_err(AppError::Git)?
        .to_owned();

    let merge_ref = repo
        .branch_upstream_merge(local_ref)
        .ok()
        .and_then(|buf| buf.as_str().ok().map(str::to_owned))
        .unwrap_or_else(|| local_ref.to_owned());

    let tag_ref = format!("refs/tags/v{version}");
    let branch_refspec = format!("{local_ref}:{merge_ref}");

    let mut remote = repo
        .find_remote(&remote_name)
        .map_err(|e| AppError::PushFailed(format!("remote '{remote_name}' not found: {e}")))?;

    let config = repo.config().map_err(AppError::Git)?;
    let mut attempts = CredAttempt::default();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| {
        git_credentials(url, username_from_url, allowed, &config, &mut attempts)
    });
    callbacks.push_update_reference(|name, status| match status {
        Some(msg) => Err(git2::Error::from_str(&format!("rejected {name}: {msg}"))),
        None => Ok(()),
    });

    let mut opts = PushOptions::new();
    opts.remote_callbacks(callbacks);

    remote
        .push(
            &[branch_refspec.as_str(), tag_ref.as_str()],
            Some(&mut opts),
        )
        .map_err(|e| AppError::PushFailed(e.to_string()))?;

    Ok(())
}

#[derive(Default)]
struct CredAttempt {
    tried_username: bool,
    tried_agent: bool,
    key_index: usize,
    tried_helper: bool,
    tried_default: bool,
}

fn git_credentials(
    url: &str,
    username_from_url: Option<&str>,
    allowed: CredentialType,
    config: &git2::Config,
    state: &mut CredAttempt,
) -> Result<Cred, git2::Error> {
    if allowed.contains(CredentialType::USERNAME) && !state.tried_username {
        state.tried_username = true;
        return Cred::username(username_from_url.unwrap_or("git"));
    }

    if allowed.contains(CredentialType::SSH_KEY) {
        let username = username_from_url.unwrap_or("git");
        if !state.tried_agent {
            state.tried_agent = true;
            if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
        }

        let keys = ssh_identity_files();
        while state.key_index < keys.len() {
            let key = &keys[state.key_index];
            state.key_index += 1;
            if let Ok(cred) = Cred::ssh_key(username, None, key, None) {
                return Ok(cred);
            }
        }
    }

    if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) && !state.tried_helper {
        state.tried_helper = true;
        return Cred::credential_helper(config, url, username_from_url);
    }

    if allowed.contains(CredentialType::DEFAULT) && !state.tried_default {
        state.tried_default = true;
        return Cred::default();
    }

    Err(git2::Error::from_str("authentication failed"))
}

fn ssh_identity_files() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
    else {
        return Vec::new();
    };

    let ssh = home.join(".ssh");
    ["id_ed25519", "id_ecdsa", "id_rsa"]
        .into_iter()
        .map(|name| ssh.join(name))
        .filter(|path| path.exists())
        .collect()
}

fn add_tag(repo: &Repository, version: &str) -> Result<(), AppError> {
    let obj = repo
        .head()
        .map_err(AppError::Git)?
        .peel_to_commit()
        .map_err(AppError::Git)?
        .into_object();

    let tagger = repo.signature().map_err(|_| AppError::NoSignature)?;

    repo.tag(
        &format!("v{version}"),
        &obj,
        &tagger,
        &format!("Release: v{version}"),
        true,
    )
    .map_err(AppError::Git)?;

    Ok(())
}

fn commit_changes(
    repo: &Repository,
    (version, files): (&str, Vec<String>),
) -> Result<(), AppError> {
    let message = format!("Release: v{version}");

    let mut index = repo.index().map_err(AppError::Git)?;
    index
        .add_all(files.iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(AppError::Git)?;
    index.write().map_err(AppError::Git)?;

    let tree_id = index.write_tree().map_err(AppError::Git)?;
    let tree = repo.find_tree(tree_id).map_err(AppError::Git)?;
    let sig = repo.signature().map_err(|_| AppError::NoSignature)?;

    let parent = match repo.head() {
        Ok(head) => Some(head.peel_to_commit().map_err(AppError::Git)?),
        Err(_) => None,
    };
    let parents: Vec<&git2::Commit> = parent.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &parents)
        .map_err(AppError::Git)?;

    Ok(())
}
