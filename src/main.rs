use std::{fs, path::PathBuf};

use clap::{
    ValueEnum, arg,
    builder::{EnumValueParser, PossibleValue},
    command, value_parser,
};
use git2::Repository;
use toml_edit::DocumentMut;

fn main() {
    let matches = command!()
        .arg(arg!([kind] "The release version e.g major.").required(true).value_parser(EnumValueParser::<ReleaseKind>::new()))
        .arg(arg!(
            -r --repo <PATH> "Specify the git repo."
        ).value_parser(value_parser!(PathBuf)))
        .after_help("This util helps streamline the release process. Calling git release increments the tag.")
        .get_matches();

    let release_type = matches
        .get_one::<ReleaseKind>("kind")
        .expect("This has been validated.");

    let default_path = PathBuf::from("./").to_path_buf();
    let directory = matches
        .get_one::<PathBuf>("repo")
        .or(Some(&default_path))
        .expect("The path should exist here.");

    let repo = Repository::open(directory).unwrap();

    let next_version = bump_package_version(release_type, directory.to_path_buf());

    commit_changes(&repo, &next_version);
    add_tag_to_repo(&repo, &next_version);

    println!("Version {next_version} has been released.")
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

#[derive(Debug, Clone)]
enum ReleaseKind {
    Major,
    Minor,
    Patch,
}

impl ValueEnum for ReleaseKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[ReleaseKind::Major, ReleaseKind::Minor, ReleaseKind::Patch]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            ReleaseKind::Major => PossibleValue::new("major").help("Select the major option."),
            ReleaseKind::Minor => PossibleValue::new("minor").help("Select the major option."),
            ReleaseKind::Patch => PossibleValue::new("patch").help("Select the major option."),
        })
    }
}

fn bump_package_version(release_type: &ReleaseKind, directory: PathBuf) -> std::string::String {
    let content = fs::read_to_string(directory.join("Cargo.toml")).unwrap();
    let mut doc: DocumentMut = content.parse().unwrap();
    let current_version = doc["package"]["version"].as_str().unwrap();
    let next_version = get_next_version(current_version, release_type);

    doc["package"]["version"] = toml_edit::value(&next_version);

    fs::write(directory.join("Cargo.toml"), doc.to_string()).unwrap();

    next_version
}

fn commit_changes(repo: &Repository, next_version: &str) {
    let commit_message = format!("Release: v{next_version}");

    let mut index = repo.index().unwrap();
    index
        .add_all(["Cargo.toml"].iter(), git2::IndexAddOption::DEFAULT, None)
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

fn get_next_version(current_version: &str, kind: &ReleaseKind) -> String {
    let mut parts = current_version
        .split(".")
        .filter_map(|s| s.parse::<u32>().ok());
    let (mut major, mut minor, mut patch) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );

    match kind {
        ReleaseKind::Major => {
            major += 1;
            minor = 0;
            patch = 0
        }
        ReleaseKind::Minor => {
            minor += 1;
            patch = 0
        }
        ReleaseKind::Patch => patch += 1,
    }

    format!("{major}.{minor}.{patch}")
}

#[cfg(test)]
mod test {
    use crate::{ReleaseKind, get_next_version};

    #[test]
    fn test_get_next_version() {
        let current_version = "2.4.0";
        let major_version = get_next_version(current_version, &ReleaseKind::Major);
        assert_eq!(major_version, "3.0.0");

        let minor_version = get_next_version(current_version, &ReleaseKind::Minor);
        assert_eq!(minor_version, "2.5.0");

        let patch_version = get_next_version(current_version, &ReleaseKind::Patch);
        assert_eq!(patch_version, "2.4.1");
    }
}
