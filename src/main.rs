use std::path::PathBuf;

use git_release::ecosystem::{
    cli,
    types::{Ecosystem, EcosystemType, ReleaseKind, RustEcosystem},
};
use git2::Repository;

fn main() {
    let matches = cli::cli();

    let release_type = matches
        .get_one::<ReleaseKind>("kind")
        .expect("This has been validated.");

    let default_path = PathBuf::from("./").to_path_buf();
    let directory = matches
        .get_one::<PathBuf>("repo")
        .or(Some(&default_path))
        .expect("The path should exist here.");
    let repo = Repository::open(directory).unwrap();

    if let Some(ecosystem) = EcosystemType::detect(directory) {
        let next_version = match ecosystem {
            EcosystemType::Cargo => {
                let ec = RustEcosystem::new(directory.to_path_buf(), release_type);
                Some(ec.bump_package_version())
            }
            EcosystemType::Npm => None,
        }
        .unwrap();

        commit_changes(&repo, &next_version);
        add_tag_to_repo(&repo, &next_version);

        println!("Version {next_version} has been released.")
    } else {
        println!("Package ecosystem not supported at the moment.")
    }
}

fn add_tag_to_repo(repo: &Repository, next_version: &str) {
    let obj = repo.head().unwrap().peel_to_commit().unwrap().into_object();
    let tagger = repo.signature().unwrap();
    repo.tag(
        &format!("v{next_version}"),
        &obj,
        &tagger,
        &format!("Release: v{next_version}"),
        true,
    )
    .unwrap();
}

fn commit_changes(repo: &Repository, next_version: &str) {
    let commit_message = format!("Release: v{next_version}");

    let mut index = repo.index().unwrap();
    index
        .add_all(
            ["Cargo.toml", "Cargo.lock"].iter(),
            git2::IndexAddOption::DEFAULT,
            None,
        )
        .unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let sig = repo.signature().unwrap();

    let parent_commit = match repo.head() {
        Ok(head_ref) => Some(head_ref.peel_to_commit().unwrap()),
        Err(_) => None, // Initial commit has no parents
    };

    let parents = match &parent_commit {
        Some(c) => vec![c],
        None => vec![],
    };

    repo.commit(
        Some("HEAD"),    // Updates HEAD to point to this new commit
        &sig,            // Author
        &sig,            // Committer
        &commit_message, // Message
        &tree,           // Staged tree
        &parents,        // Parent commits
    )
    .unwrap();
}
