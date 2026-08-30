use std::{fs, path::PathBuf, process::Command};

use toml_edit::DocumentMut;

use crate::ecosystem::types::{Ecosystem, ReleaseKind};

pub struct RustEcosystem<'a> {
    directory: PathBuf,
    release_type: &'a ReleaseKind,
}

impl<'a> RustEcosystem<'a> {}

impl<'a> Ecosystem<'a> for RustEcosystem<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self {
        Self {
            directory,
            release_type,
        }
    }

    fn get_current_version(&self) -> String {
        let content = fs::read_to_string(self.directory.join("Cargo.toml")).unwrap();
        let doc: DocumentMut = content.parse().unwrap();
        let current_version = doc["package"]["version"].as_str().unwrap();
        current_version.into()
    }

    fn bump_package_version(&self) -> (String, Vec<String>) {
        let content = fs::read_to_string(self.directory.join("Cargo.toml")).unwrap();
        let mut doc: DocumentMut = content.parse().unwrap();
        let next_version = self.get_next_version();

        doc["package"]["version"] = toml_edit::value(&next_version);

        fs::write(self.directory.join("Cargo.toml"), doc.to_string()).unwrap();

        (next_version, self.sync_lockfile())
    }

    fn sync_lockfile(&self) -> Vec<String> {
        Command::new("cargo")
            .arg("check")
            .arg("--workspace")
            .current_dir(self.directory.clone())
            .status()
            .unwrap();

        vec!["Cargo.toml".to_owned(), "Cargo.lock".to_owned()]
    }

    fn get_directory(&self) -> &PathBuf {
        &self.directory
    }

    fn get_release_type(&self) -> &ReleaseKind {
        self.release_type
    }
}
