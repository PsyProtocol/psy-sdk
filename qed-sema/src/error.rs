use ariadne::{ColorGenerator, Label, Report, ReportKind};
use qed_ast::{IdentId, Span};
use thiserror::Error;

use crate::TypeId;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("{0:?}")]
    CommonError(#[from] qed_common::Error),
    #[error("type mismatch")]
    TypeMismatch {
        span: Span,
        expected: Vec<TypeId>,
        found: TypeId,
    },
    #[error("unresolved path")]
    UnresolvedPath { span: Span, resolved_path: String },
    #[error("invalid path segment")]
    InvalidPathSegment { span: Span, segment: IdentId },
    #[error("unresolved value")]
    UnresolvedVariable {
        span: Span,
        resolved_variable: IdentId,
    },
    #[error("unresolved type")]
    UnresolvedType { span: Span, resolved_type: IdentId },
    #[error("variable already defined")]
    VariableAlreadyDefined { span: Span, variable: IdentId },
    #[error("undefined variable")]
    UndefinedVariable { span: Span, variable: IdentId },
    #[error("immutable variable")]
    ImmutableVariable { span: Span, variable: IdentId },
    #[error("unresolved member")]
    UnresolvedMember { span: Span, member_name: IdentId },
    #[error("unresolved trait method")]
    UnresolvedTraitMethod {
        method_span: Span,
        method_name: IdentId,
        trait_name: IdentId,
    },
    #[error("function parameter mismatch")]
    FunctionParameterMismatch {
        span: Span,
        expected: TypeId,
        found: TypeId,
    },
    #[error("generic parameter mismatch")]
    GenericParameterMismatch {
        span: Span,
        expected: String,
        found: String,
    },
    #[error("invalid function call")]
    InvalidFunctionCall {
        span: Span,
        method_name: TypeId,
        expected: String,
        found: String,
    },
    #[error("invalid return")]
    InvalidReturn { span: Span, message: String },
    #[error("unreachable expression")]
    UnreachableExpression { span: Span },
    #[error("invalid self parameter")]
    InvalidSelfParameter { span: Span, message: IdentId },
    #[error("type already defined")]
    TypeAlreadyDefined { span: Span, type_name: IdentId },
    #[error("member not public")]
    MemberNotPublic {
        span: Span,
        ty: TypeId,
        field: IdentId,
    },
    #[error("module not public")]
    ModuleNotPublic { span: Span, module: IdentId },
    #[error("type not public")]
    TypeNotPublic { span: Span, ty: TypeId },
    #[error("trait already implemented")]
    TraitAlreadyImplemented {
        span: Span,
        trait_ty: TypeId,
        ty: TypeId,
    },
    #[error("trait method unimplemented")]
    TraitMethodUnimplemented {
        span: Span,
        trait_ty: TypeId,
        ty: TypeId,
        method: IdentId,
    },
    #[error("Method has no body")]
    MethodHasNoBody {
        span: Span,
        ty: TypeId,
        method: IdentId,
    },
    #[error("Function has no body")]
    FunctionHasNoBody { span: Span, function: IdentId },
    #[error("DuplicatedMethod")]
    DuplicatedMethod {
        span: Span,
        ty: TypeId,
        method: IdentId,
    },
    #[error("index out of bounds")]
    IndexOutOfBounds {
        span: Span,
        index: usize,
        length: usize,
    },
    #[error("invalid cast")]
    InvalidCast {
        span: Span,
        expected: String,
        found: String,
    },
    #[error("no parent module")]
    NoParentModule { span: Span },
    #[error("module not found")]
    ModuleNotFound { span: Span, module: IdentId },
    #[error("unreachable code")]
    DuplicateWildcard { span: Span },
    #[error("Incomplete Match")]
    IncompleteMatch { span: Span, message: String },
}

pub type Result<T> = std::result::Result<T, Error>;
