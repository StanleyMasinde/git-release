use std::{
    io::Write,
    path::PathBuf,
    process::{self, Command, Stdio},
};

use git_release::{
    ecosystem::{
        cli,
        recipes::{cargo::CargoRecipe, npm::NpmRecipe},
        types::{Ecosystem, EcosystemType, ReleaseKind},
    },
    errors::AppError,
};
use git2::{Cred, CredentialType, ObjectType, Oid, PushOptions, RemoteCallbacks, Repository};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let matches = cli::cli().get_matches();

    let release_type = matches
        .get_one::<ReleaseKind>("kind")
        .expect("required argument; validated by clap");

    let directory = matches
        .get_one::<PathBuf>("repo")
        .expect("defaulted by clap");

    let repo = Repository::open(directory)
        .map_err(|_| AppError::RepoNotFound(directory.display().to_string()))?;

    let should_push = matches.get_flag("push");
    let should_sign = matches.get_flag("sign");

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

    commit_changes(&repo, (&next_version, files), should_sign)?;
    add_tag(&repo, &next_version, should_sign)?;

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

fn add_tag(repo: &Repository, version: &str, sign: bool) -> Result<(), AppError> {
    let obj = repo
        .head()
        .map_err(AppError::Git)?
        .peel_to_commit()
        .map_err(AppError::Git)?
        .into_object();

    let tagger = repo.signature().map_err(|_| AppError::NoSignature)?;
    let name = format!("v{version}");
    let message = format!("Release: v{version}");

    if sign {
        let payload = annotated_tag_payload(
            &name,
            obj.id(),
            obj.kind().unwrap_or(ObjectType::Commit).str(),
            &tagger,
            &message,
        );
        let signature = gpg_sign(repo, &payload)?;
        let mut signed = payload;
        signed.push_str(&signature);
        if !signed.ends_with('\n') {
            signed.push('\n');
        }

        let oid = repo
            .odb()
            .map_err(AppError::Git)?
            .write(ObjectType::Tag, signed.as_bytes())
            .map_err(AppError::Git)?;
        repo.reference(&format!("refs/tags/{name}"), oid, true, &message)
            .map_err(AppError::Git)?;
    } else {
        repo.tag(&name, &obj, &tagger, &message, true)
            .map_err(AppError::Git)?;
    }

    Ok(())
}

fn commit_changes(
    repo: &Repository,
    (version, files): (&str, Vec<String>),
    sign: bool,
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

    if sign {
        let buf = repo
            .commit_create_buffer(&sig, &sig, &message, &tree, &parents)
            .map_err(AppError::Git)?;
        let commit_content = std::str::from_utf8(&buf)
            .map_err(|_| AppError::SignFailed("commit buffer is not valid UTF-8".to_owned()))?;
        let signature = gpg_sign(repo, commit_content)?;
        let oid = repo
            .commit_signed(commit_content, &signature, None)
            .map_err(AppError::Git)?;
        update_head(repo, oid, &message)?;
    } else {
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &parents)
            .map_err(AppError::Git)?;
    }

    Ok(())
}

fn update_head(repo: &Repository, oid: Oid, message: &str) -> Result<(), AppError> {
    match repo.head() {
        Ok(mut head) => {
            if head.is_branch() {
                head.set_target(oid, message).map_err(AppError::Git)?;
            } else {
                repo.set_head_detached(oid).map_err(AppError::Git)?;
            }
        }
        Err(err) if err.code() == git2::ErrorCode::UnbornBranch => {
            let head = repo.find_reference("HEAD").map_err(AppError::Git)?;
            let branch_ref = head
                .symbolic_target()
                .map_err(AppError::Git)?
                .ok_or_else(|| AppError::SignFailed("HEAD is unborn but not symbolic".to_owned()))?
                .to_owned();
            repo.reference(&branch_ref, oid, true, message)
                .map_err(AppError::Git)?;
        }
        Err(err) => return Err(AppError::Git(err)),
    }
    Ok(())
}

fn annotated_tag_payload(
    name: &str,
    target_id: Oid,
    target_type: &str,
    tagger: &git2::Signature<'_>,
    message: &str,
) -> String {
    let mut buf = format!(
        "object {target_id}\ntype {target_type}\ntag {name}\ntagger {}\n\n{message}",
        format_git_signature(tagger)
    );
    if !buf.ends_with('\n') {
        buf.push('\n');
    }
    buf
}

fn format_git_signature(sig: &git2::Signature<'_>) -> String {
    let time = sig.when();
    let offset = time.offset_minutes().unsigned_abs();
    format!(
        "{} <{}> {} {}{:02}{:02}",
        sig.name().unwrap_or(""),
        sig.email().unwrap_or(""),
        time.seconds(),
        time.sign(),
        offset / 60,
        offset % 60
    )
}

fn gpg_sign(repo: &Repository, payload: &str) -> Result<String, AppError> {
    let config = repo.config().ok();
    if let Some(format) = config
        .as_ref()
        .and_then(|c| c.get_string("gpg.format").ok())
    {
        if format.eq_ignore_ascii_case("ssh") {
            return Err(AppError::SignFailed(
                "gpg.format=ssh is not supported; --sign uses OpenPGP via gpg".to_owned(),
            ));
        }
    }

    let program = config
        .as_ref()
        .and_then(|c| c.get_string("gpg.program").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "gpg".to_owned());

    let mut cmd = Command::new(&program);
    cmd.args(["--detach-sign", "--armor"]);
    if let Some(key) = config
        .as_ref()
        .and_then(|c| c.get_string("user.signingkey").ok())
        .filter(|s| !s.is_empty())
    {
        cmd.args(["--local-user", &key]);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::SignFailed(format!("could not run `{program}`: {e}")))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::SignFailed("failed to open gpg stdin".to_owned()))?;
        stdin
            .write_all(payload.as_bytes())
            .map_err(|e| AppError::SignFailed(format!("failed to write to gpg: {e}")))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| AppError::SignFailed(format!("failed to wait for `{program}`: {e}")))?;

    if !output.status.success() {
        return Err(AppError::SignFailed(format!(
            "`{program}` exited with status {}",
            output.status
        )));
    }

    let signature = String::from_utf8(output.stdout)
        .map_err(|_| AppError::SignFailed("gpg produced a non-UTF-8 signature".to_owned()))?;

    if !signature.contains("BEGIN PGP SIGNATURE") {
        return Err(AppError::SignFailed(
            "gpg produced no signature (is user.signingkey set and is the secret key available?)"
                .to_owned(),
        ));
    }

    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Signature, Time};

    #[test]
    fn git_signature_formats_offset() {
        let sig = Signature::new("Ada", "ada@example.com", &Time::new(1_700_000_000, 180)).unwrap();
        assert_eq!(
            format_git_signature(&sig),
            "Ada <ada@example.com> 1700000000 +0300"
        );

        let sig =
            Signature::new("Ada", "ada@example.com", &Time::new(1_700_000_000, -270)).unwrap();
        assert_eq!(
            format_git_signature(&sig),
            "Ada <ada@example.com> 1700000000 -0430"
        );
    }

    #[test]
    fn annotated_tag_payload_ends_with_newline() {
        let sig = Signature::new("Ada", "ada@example.com", &Time::new(1_700_000_000, 0)).unwrap();
        let oid = Oid::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let payload = annotated_tag_payload("v1.2.3", oid, "commit", &sig, "Release: v1.2.3");
        assert!(payload.ends_with('\n'));
        assert!(payload.starts_with("object aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n"));
        assert!(payload.contains("type commit\n"));
        assert!(payload.contains("tag v1.2.3\n"));
        assert!(payload.contains("tagger Ada <ada@example.com> 1700000000 +0000\n"));
        assert!(payload.contains("\n\nRelease: v1.2.3\n"));
    }
}
