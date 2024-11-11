use std::fmt::Display;

use crate::ParseError;

#[derive(Debug)]
pub enum Error<'a> {
    Lexical(ParseError<'a>),
    CycleDependency,
    IoError(std::io::Error),
    FileUnresolved,
    InvalidModuleName,
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lexical(err) => write!(f, "{}", err),
            Self::CycleDependency => write!(f, "Cycle dependency detected"),
            Self::IoError(err) => write!(f, "{}", err),
            Self::FileUnresolved => write!(f, "File could not be resolved"),
            Self::InvalidModuleName => write!(f, "Invalid module name"),
        }
    }
}

impl<'a> From<ParseError<'a>> for Error<'a> {
    fn from(err: ParseError<'a>) -> Self {
        Self::Lexical(err)
    }
}

impl<'a> From<std::io::Error> for Error<'a> {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub type Result<'a, T> = std::result::Result<T, Error<'a>>;
