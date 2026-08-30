use std::path::{Path, PathBuf};

use clap::{ValueEnum, builder::PossibleValue};

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

pub trait Ecosystem<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self;
    fn get_current_version(&self) -> String;
    fn bump_package_version(&self) -> (String, Vec<String>);
    fn sync_lockfile(&self) -> Vec<String>;
}
