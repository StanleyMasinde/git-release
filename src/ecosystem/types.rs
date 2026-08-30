use std::path::{Path, PathBuf};

use clap::{ValueEnum, builder::PossibleValue};

use crate::ecosystem::next_version::get_next_version;

#[derive(Debug, Clone)]
pub enum ReleaseKind {
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
            ReleaseKind::Major => PossibleValue::new("major").help("Bump major version (X.0.0)"),
            ReleaseKind::Minor => PossibleValue::new("minor").help("Bump minor version (x.Y.0)"),
            ReleaseKind::Patch => PossibleValue::new("patch").help("Bump patch version (x.y.Z)"),
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

pub trait Ecosystem<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self;
    fn get_directory(&self) -> &PathBuf;
    fn get_release_type(&self) -> &ReleaseKind;
    fn get_current_version(&self) -> String;
    fn bump_package_version(&self) -> (String, Vec<String>);
    fn sync_lockfile(&self) -> Vec<String>;

    fn get_next_version(&self) -> String {
        get_next_version(&self.get_current_version(), self.get_release_type())
    }
}
