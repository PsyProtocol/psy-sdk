use crate::LalrpopError;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error<'a> {
    #[error("{0}")]
    LexicalError(#[from] qed_lexer::Error),
    #[error("{0}")]
    Lexical(Box<LalrpopError<'a>>),
    #[error("{0}")]
    CommonError(#[from] qed_common::Error),
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("File could not be resolved")]
    FileUnresolved,
    #[error("Invalid module name")]
    InvalidModuleName,
    #[error("Extern function can only be defined in std")]
    ExternFnNotInStd,
    #[error("Missing function body")]
    FunctionBodyMissing,
    #[error("Invalid self parameter")]
    InvalidSelfParameter,
}

impl<'a> From<LalrpopError<'a>> for Error<'a> {
    fn from(err: LalrpopError<'a>) -> Self {
        Self::Lexical(Box::new(err))
    }
}

pub type Result<'a, T> = std::result::Result<T, Error<'a>>;
