use qed_sema::Error as SemaError;
use std::io::Error as IoError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("io error: {0}")]
    IoError(#[from] IoError),
    #[error("sema error: {0}")]
    SemaError(#[from] SemaError),
    #[error("undefined function")]
    UndefinedFunction,
    #[error("uncertain loop condition")]
    UncertainLoopCondition,
}

pub type Result<T> = std::result::Result<T, Error>;
