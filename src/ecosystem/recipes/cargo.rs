use std::{fs, path::PathBuf, process::Command};

use toml_edit::DocumentMut;

use crate::ecosystem::types::{Ecosystem, ReleaseKind};
use crate::errors::AppError;

pub struct CargoRecipe<'a> {
    directory: PathBuf,
    release_type: &'a ReleaseKind,
}

impl<'a> Ecosystem<'a> for CargoRecipe<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self {
        Self {
            directory,
            release_type,
        }
    }

    fn get_current_version(&self) -> Result<String, AppError> {
        let path = self.directory.join("Cargo.toml");
        let content = fs::read_to_string(&path).map_err(|e| AppError::ReadFile {
            path: path.display().to_string(),
            source: e,
        })?;

        let doc: DocumentMut =
            content
                .parse()
                .map_err(|e: toml_edit::TomlError| AppError::ParseFile {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?;

        let version = doc["package"]["version"]
            .as_str()
            .ok_or_else(|| AppError::MissingField {
                field: "package.version".to_owned(),
                path: path.display().to_string(),
            })?
            .to_owned();

        Ok(version)
    }

    fn bump_package_version(&self) -> Result<(String, Vec<String>), AppError> {
        let path = self.directory.join("Cargo.toml");
        let content = fs::read_to_string(&path).map_err(|e| AppError::ReadFile {
            path: path.display().to_string(),
            source: e,
        })?;

        let mut doc: DocumentMut =
            content
                .parse()
                .map_err(|e: toml_edit::TomlError| AppError::ParseFile {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?;

        let next_version = self.get_next_version()?;
        doc["package"]["version"] = toml_edit::value(&next_version);

        fs::write(&path, doc.to_string()).map_err(|e| AppError::WriteFile {
            path: path.display().to_string(),
            source: e,
        })?;

        let staged = self.sync_lockfile()?;
        Ok((next_version, staged))
    }

    fn sync_lockfile(&self) -> Result<Vec<String>, AppError> {
        let status = Command::new("cargo")
            .arg("check")
            .arg("--workspace")
            .current_dir(&self.directory)
            .status()
            .map_err(|e| AppError::CommandFailed {
                cmd: "cargo check --workspace".to_owned(),
                reason: e.to_string(),
            })?;

        if !status.success() {
            return Err(AppError::CommandFailed {
                cmd: "cargo check --workspace".to_owned(),
                reason: format!("exited with status {status}"),
            });
        }

        Ok(vec!["Cargo.toml".to_owned(), "Cargo.lock".to_owned()])
    }

    fn get_directory(&self) -> &PathBuf {
        &self.directory
    }

    fn get_release_type(&self) -> &ReleaseKind {
        self.release_type
    }
}
