use qed_dargo_toml::errors::ManifestError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CliError {
    #[error("{0}")]
    Generic(String),
    /// Error from Manifest
    #[error(transparent)]
    ManifestError(#[from] ManifestError),
}
