use qed_dargo_toml::errors::ManifestError;
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
    /// Error related to deserialization
    #[error("Failed to deserialize artifact from JSON")]
    DeserializationError(#[from] serde_json::Error),
}
