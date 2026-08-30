use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("No git repository found at '{0}'")]
    RepoNotFound(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Could not read '{path}': {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },

    #[error("Could not write '{path}': {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },

    #[error("Could not parse '{path}': {reason}")]
    ParseFile { path: String, reason: String },

    #[error("'{field}' field missing or invalid in '{path}'")]
    MissingField { field: String, path: String },

    #[error("Failed to run `{cmd}`: {reason}")]
    CommandFailed { cmd: String, reason: String },

    #[error("Git user.name and user.email must be set in your git config")]
    NoSignature,
}
