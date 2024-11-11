use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum Error {
    #[error("type mismatch")]
    TypeMismatch,
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
    #[error("unresolved member")]
    UnresolvedMember,
    #[error("function parameter mismatch")]
    FunctionParameterMismatch,
    #[error("invalid function call")]
    InvalidFunctionCall,
}

pub type Result<T> = std::result::Result<T, Error>;
