use std::fmt::Display;

use crate::LalrpopError;

#[derive(Debug)]
pub enum Error<'a> {
    Lexical(Box<LalrpopError<'a>>),
    CycleDependency,
    IoError(std::io::Error),
    FileUnresolved,
    InvalidModuleName,
    ExternFnNotInStd,
    FunctionBodyMissing,
    InvalidSelfParameter,
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lexical(err) => write!(f, "{}", err),
            Self::CycleDependency => write!(f, "Cycle dependency detected"),
            Self::IoError(err) => write!(f, "{}", err),
            Self::FileUnresolved => write!(f, "File could not be resolved"),
            Self::InvalidModuleName => write!(f, "Invalid module name"),
            Self::ExternFnNotInStd => {
                write!(f, "Extern function can only be defined in std")
            }
            Self::FunctionBodyMissing => {
                write!(f, "missing function body")
            }
            Self::InvalidSelfParameter => {
                write!(f, "Invalid self parameter")
            }
        }
    }
}

impl<'a> From<LalrpopError<'a>> for Error<'a> {
    fn from(err: LalrpopError<'a>) -> Self {
        Self::Lexical(Box::new(err))
    }
}

impl<'a> From<std::io::Error> for Error<'a> {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub type Result<'a, T> = std::result::Result<T, Error<'a>>;
