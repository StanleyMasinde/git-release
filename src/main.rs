use std::{path::PathBuf, process};

use git_release::{
    ecosystem::{
        cli,
        recipes::{cargo::CargoRecipe, npm::NpmRecipe},
        types::{Ecosystem, EcosystemType, ReleaseKind},
    },
    errors::AppError,
};
use git2::Repository;

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

    let default_path = PathBuf::from("./");
    let directory = matches.get_one::<PathBuf>("repo").unwrap_or(&default_path);

    let repo = Repository::open(directory)
        .map_err(|_| AppError::RepoNotFound(directory.display().to_string()))?;

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

    println!("Released v{next_version}. Run `git push --follow-tags` to publish.");

    Ok(())
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
