use std::{collections::HashSet, fs, path::PathBuf, process::Command};

use serde_json::Value;

use crate::ecosystem::types::{Ecosystem, ReleaseKind};
use crate::errors::AppError;

pub struct NpmRecipe<'a> {
    directory: PathBuf,
    release_type: &'a ReleaseKind,
}

impl<'a> Ecosystem<'a> for NpmRecipe<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self {
        Self {
            directory,
            release_type,
        }
    }

    fn get_current_version(&self) -> Result<String, AppError> {
        let path = self.directory.join("package.json");
        let data = fs::read_to_string(&path).map_err(|e| AppError::ReadFile {
            path: path.display().to_string(),
            source: e,
        })?;

        let json: Value = serde_json::from_str(&data).map_err(|e| AppError::ParseFile {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        json["version"]
            .as_str()
            .ok_or_else(|| AppError::MissingField {
                field: "version".to_owned(),
                path: path.display().to_string(),
            })
            .map(|s| s.to_owned())
    }

    fn bump_package_version(&self) -> Result<(String, Vec<String>), AppError> {
        let path = self.directory.join("package.json");
        let data = fs::read_to_string(&path).map_err(|e| AppError::ReadFile {
            path: path.display().to_string(),
            source: e,
        })?;

        let mut json: Value = serde_json::from_str(&data).map_err(|e| AppError::ParseFile {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        let next_version = self.get_next_version()?;
        json["version"] = Value::String(next_version.clone());

        let updated = serde_json::to_string_pretty(&json).map_err(|e| AppError::ParseFile {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        fs::write(&path, updated).map_err(|e| AppError::WriteFile {
            path: path.display().to_string(),
            source: e,
        })?;

        let staged = self.sync_lockfile()?;
        Ok((next_version, staged))
    }

    fn sync_lockfile(&self) -> Result<Vec<String>, AppError> {
        let files: HashSet<String> = fs::read_dir(&self.directory)
            .map_err(|e| AppError::ReadFile {
                path: self.directory.display().to_string(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        let mut staged = vec!["package.json".to_owned()];

        if files.contains("pnpm-lock.yaml") {
            let status = Command::new("pnpm")
                .args(["install", "--lockfile-only"])
                .current_dir(&self.directory)
                .status()
                .map_err(|e| AppError::CommandFailed {
                    cmd: "pnpm install --lockfile-only".to_owned(),
                    reason: e.to_string(),
                })?;
            if !status.success() {
                return Err(AppError::CommandFailed {
                    cmd: "pnpm install --lockfile-only".to_owned(),
                    reason: format!("exited with status {status}"),
                });
            }
            staged.push("pnpm-lock.yaml".to_owned());
        } else if files.contains("package-lock.json") {
            let status = Command::new("npm")
                .args(["install", "--package-lock-only"])
                .current_dir(&self.directory)
                .status()
                .map_err(|e| AppError::CommandFailed {
                    cmd: "npm install --package-lock-only".to_owned(),
                    reason: e.to_string(),
                })?;
            if !status.success() {
                return Err(AppError::CommandFailed {
                    cmd: "npm install --package-lock-only".to_owned(),
                    reason: format!("exited with status {status}"),
                });
            }
            staged.push("package-lock.json".to_owned());
        } else if files.contains("yarn.lock") {
            let status = Command::new("yarn")
                .args(["install", "--pure-lockfile", "--offline"])
                .current_dir(&self.directory)
                .status()
                .map_err(|e| AppError::CommandFailed {
                    cmd: "yarn install --pure-lockfile --offline".to_owned(),
                    reason: e.to_string(),
                })?;
            if !status.success() {
                return Err(AppError::CommandFailed {
                    cmd: "yarn install --pure-lockfile --offline".to_owned(),
                    reason: format!("exited with status {status}"),
                });
            }
            staged.push("yarn.lock".to_owned());
        }

        Ok(staged)
    }

    fn get_directory(&self) -> &PathBuf {
        &self.directory
    }

    fn get_release_type(&self) -> &ReleaseKind {
        self.release_type
    }
}
