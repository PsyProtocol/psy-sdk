use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("Unknown")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;
