use std::{fs, path::PathBuf};

use serde::Deserialize;

use crate::ecosystem::types::{Ecosystem, ReleaseKind};


#[derive(Deserialize)]
struct PackageVersion {
    version: String,
}

struct ComposerEcosystem<'a> {
    directory: PathBuf,
    release_type: &'a ReleaseKind,
}

impl<'a> Ecosystem<'a> for ComposerEcosystem<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self {
        Self {
            directory,
            release_type,
        }
    }

    fn get_directory(&self) -> &PathBuf {
        &self.directory
    }

    fn get_release_type(&self) -> &ReleaseKind {
        self.release_type
    }

    fn get_current_version(&self) -> String {
        let file = fs::read_to_string(self.directory.join("composer.json")).unwrap();
        let package: PackageVersion = serde_json::from_str(&file).unwrap();

        package.version
    }

    fn bump_package_version(&self) -> (String, Vec<String>) {
        todo!()
    }

    fn sync_lockfile(&self) -> Vec<String> {
        todo!()
    }
}
