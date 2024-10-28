use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Default, Debug, Clone, PartialEq)]
pub enum Error {
    #[error("invalid integer: `{0}`")]
    InvalidInteger(ParseIntError),
    #[error("invalid token")]
    #[default]
    InvalidToken,
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Error::InvalidInteger(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
