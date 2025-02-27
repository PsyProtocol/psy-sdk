use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    AnyError(#[from] anyhow::Error),
    #[error("type mismatch")]
    TypeMismatch,
    #[error("unresolved path")]
    UnresolvedPath,
    #[error("unresolved value")]
    UnresolvedVariable,
    #[error("unresolved type")]
    UnresolvedType,
    #[error("unresolved use")]
    UnresolvedUse,
    #[error("variable already defined")]
    VariableAlreadyDefined,
    #[error("undefined variable")]
    UndefinedVariable,
    #[error("immutable variable")]
    ImmutableVariable,
    #[error("unresolved implementor")]
    UnresolvedImplementor,
    #[error("unresolved trait")]
    UnresolvedTrait,
    #[error("unresolved member")]
    UnresolvedMember,
    #[error("function parameter mismatch")]
    FunctionParameterMismatch,
    #[error("invalid function call")]
    InvalidFunctionCall,
    #[error("invalid return")]
    InvalidReturn,
    #[error("invalid self parameter")]
    InvalidSelfParameter,
    #[error("type already defined")]
    TypeAlreadyDefined,
    #[error("index out of bounds")]
    IndexOutOfBounds,
}

pub type Result<T> = std::result::Result<T, Error>;
