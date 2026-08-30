use std::{collections::HashSet, fs, path::PathBuf, process::Command};

use serde::Deserialize;
use serde_json::Value;

use crate::ecosystem::{
    next_version::get_next_version,
    types::{Ecosystem, ReleaseKind},
};

#[derive(Deserialize)]
struct PackageVersion {
    version: String,
}

pub struct NpmEcosystem<'a> {
    directory: PathBuf,
    release_type: &'a ReleaseKind,
}
impl<'a> Ecosystem<'a> for NpmEcosystem<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self {
        Self {
            directory,
            release_type,
        }
    }

    fn get_current_version(&self) -> String {
        let file = fs::read_to_string(self.directory.join("package.json")).unwrap();
        let package: PackageVersion = serde_json::from_str(&file).unwrap();

        package.version
    }

    fn bump_package_version(&self) -> (String, Vec<String>) {
        let file_path = self.directory.join("package.json");

        let data = fs::read_to_string(&file_path).unwrap();

        let mut json: Value = serde_json::from_str(&data).unwrap();

        let next_version = get_next_version(&self.get_current_version(), self.release_type);

        json["version"] = Value::String(next_version.clone());

        let updated_json = serde_json::to_string_pretty(&json).unwrap();

        fs::write(file_path, updated_json).unwrap();

        (next_version, self.sync_lockfile())
    }

    fn sync_lockfile(&self) -> Vec<String> {
        let files: HashSet<String> = fs::read_dir(&self.directory)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();

        let mut commitable_files: Vec<String> = vec!["package.json".to_owned()];

        if files.contains("pnpm-lock.yaml") {
            Command::new("pnpm")
                .arg("install")
                .arg("--lockfile-only")
                .current_dir(&self.directory)
                .status()
                .unwrap();

            commitable_files.push("pnpm-lock.yaml".to_owned());
        } else if files.contains("package-lock.json") {
            Command::new("npm")
                .arg("install")
                .arg("--lockfile-only")
                .current_dir(&self.directory)
                .status()
                .unwrap();

            commitable_files.push("package-lock.json".to_owned());
        } else if files.contains("yarn") {
            Command::new("yarn")
                .arg("install")
                .arg("--pure-lockfile")
                .arg("--offline")
                .current_dir(&self.directory)
                .status()
                .unwrap();

            commitable_files.push("yarn.lock".to_owned());
        }
        commitable_files
    }
}
