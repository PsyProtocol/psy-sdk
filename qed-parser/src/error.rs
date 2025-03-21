use crate::LalrpopError;
use qed_ast::Location;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum CustomError<'a> {
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

impl<'a> From<LalrpopError<'a>> for CustomError<'a> {
    fn from(err: LalrpopError<'a>) -> Self {
        Self::Lexical(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("{0}")]
    LexicalError(#[from] qed_lexer::Error),
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

    #[error("invalid token")]
    InvalidToken { location: Location },
    #[error("unrecognized eof")]
    UnrecognizedEof {
        expected: Vec<String>,
        location: Location,
    },
    #[error("unrecognized token")]
    UnrecognizedToken {
        token: String,
        expected: Vec<String>,
        location: Location,
    },
    #[error("extra token")]
    ExtraToken { token: String, location: Location },
}
