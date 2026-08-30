use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use clap::{ValueEnum, builder::PossibleValue};
use toml_edit::DocumentMut;

use crate::ecosystem::next_version::get_next_version;

#[derive(Debug, Clone)]
pub enum ReleaseKind {
    Major,
    Minor,
    Patch,
}


impl ValueEnum for ReleaseKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            ReleaseKind::Major,
            ReleaseKind::Minor,
            ReleaseKind::Patch,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            ReleaseKind::Major => PossibleValue::new("major").help("Select the major option."),
            ReleaseKind::Minor => PossibleValue::new("minor").help("Select the major option."),
            ReleaseKind::Patch => PossibleValue::new("patch").help("Select the major option."),
        })
    }
}

pub enum EcosystemType {
    Cargo,
    Npm,
}

impl EcosystemType {
    pub fn detect(dir: &Path) -> Option<Self> {
        if dir.join("Cargo.toml").exists() {
            Some(EcosystemType::Cargo)
        } else if dir.join("package.json").exists() {
            Some(EcosystemType::Npm)
        } else {
            None
        }
    }
}

pub trait Ecosystem {
    fn get_current_version(&self) -> String;
    fn bump_package_version(&self) -> String;
    fn sync_lockfile(&self);
}

pub struct RustEcosystem<'a> {
    directory: PathBuf,
    release_type: &'a ReleaseKind,
}

impl<'a> RustEcosystem<'a> {
    pub fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self {
        Self {
            directory,
            release_type,
        }
    }
}

impl<'a> Ecosystem for RustEcosystem<'a> {
    fn get_current_version(&self) -> String {
        let content = fs::read_to_string(self.directory.join("Cargo.toml")).unwrap();
        let doc: DocumentMut = content.parse().unwrap();
        let current_version = doc["package"]["version"].as_str().unwrap();
        current_version.into()
    }

    fn bump_package_version(&self) -> String {
        let content = fs::read_to_string(self.directory.join("Cargo.toml")).unwrap();
        let mut doc: DocumentMut = content.parse().unwrap();
        let current_version = &self.get_current_version();
        let next_version = get_next_version(current_version, self.release_type);

        doc["package"]["version"] = toml_edit::value(&next_version);

        fs::write(self.directory.join("Cargo.toml"), doc.to_string()).unwrap();

        next_version
    }

    fn sync_lockfile(&self) {
        Command::new("cargo")
            .arg("check")
            .arg("--workspace")
            .current_dir(self.directory.clone())
            .status()
            .unwrap();
    }
}
