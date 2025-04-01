use qed_dargo_toml::errors::ManifestError;
use std::path::PathBuf;
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Error)]
pub(crate) enum CliError {
    // #[error("{0}")]
    // Generic(String),
    /// Error from Manifest
    #[error(transparent)]
    ManifestError(#[from] ManifestError),

    /// Wrapper error for any other error type
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("Failed to deserialize artifact from JSON")]
    DeserializationError(#[from] serde_json::Error),

    #[error("Invalid package name {0}. Did you mean to use `--name`?")]
    InvalidPackageName(String),

    #[error("Error: destination {} already exists", .0.display())]
    DestinationAlreadyExists(PathBuf),
}
