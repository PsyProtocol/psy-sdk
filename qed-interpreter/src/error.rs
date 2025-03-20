use ariadne::{ColorGenerator, Label, Report, ReportKind};
use core::fmt;
use qed_ast::{Location, VisitorContext};
use qed_sema::{AstVisualizer, Error as SemaError, TypeCheckerVisitorContext};
use qedlang_core::dpn::ops::context_trait::ContextFelt;
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
    UncertainLoopCondition { loop_span: Location },
    // #[error("index out of bounds")]
    // IndexOutOfBounds,
    // #[error("type mismatch")]
    // TypeMismatch,
}

pub type Result<T> = std::result::Result<T, Error>;

fn build_report<F: Clone + From<u32> + ContextFelt, C>(
    location: Location,
    code: impl fmt::Display,
    message: impl fmt::Display,
    ctx: &TypeCheckerVisitorContext<F, C>,
) -> Result<String> {
    let file_span = ctx.program.convert_span(&location);
    let report = Report::build(ReportKind::Error, file_span.clone())
        .with_code(code)
        .with_label(Label::new(file_span).with_message(message))
        .finish();

    let mut output = Vec::new();

    report.write(
        ariadne::FnCache::new(|x: &String| {
            std::fs::read_to_string(std::path::Path::new(x.as_str()))
        }),
        &mut output,
    )?;

    Ok(String::from_utf8(output).unwrap())
}

pub fn lowering_error_to_report<F: Clone + From<u32> + ContextFelt, C>(
    error: Error,
    ctx: &TypeCheckerVisitorContext<F, C>,
) -> String {
    match error {
        Error::ParseError(error) => format!("{}", error),
        Error::IoError(error) => format!("{}", error),
        Error::SemaError(error) => match error {
            SemaError::AnyhowError(error) => format!("{}", error),
            SemaError::CommonError(error) => format!("{}", error),
            SemaError::TypeMismatch {
                location,
                expected,
                found,
            } => build_report(
                location,
                "TypeMismatch",
                format!(
                    "Expected {}, but found {}.",
                    expected
                        .into_iter()
                        .map(|ty| ctx.debug_type(ty))
                        .collect::<Vec<_>>()
                        .join(","),
                    ctx.debug_type(found)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedPath {
                location,
                resolved_path,
            } => build_report(
                location,
                "UnresolvedPath",
                format!("Unresolved path {}.", resolved_path),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidPathSegment { location, segment } => build_report(
                location,
                "InvalidPathSegment",
                format!("Invalid path segment {}.", ctx.ident(segment)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedVariable {
                location,
                resolved_variable,
            } => build_report(
                location,
                "UnresolvedVariable",
                format!("Unresolved variable {}.", ctx.ident(resolved_variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedType {
                location,
                resolved_type,
            } => build_report(
                location,
                "UnresolvedType",
                format!("Unresolved type {}.", ctx.ident(resolved_type)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::TraitAlreadyImplemented {
                location,
                trait_ty,
                ty,
            } => build_report(
                location,
                "TraitAlreadyImplemented",
                format!(
                    "Trait {} already implemented for {}.",
                    ctx.debug_type(trait_ty),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::TraitMethodUnimplemented {
                location,
                trait_ty,
                ty,
                method,
            } => build_report(
                location,
                "TraitMethodUnimplemented",
                format!(
                    "Trait {} unimplemented {} for {}.",
                    ctx.debug_type(trait_ty),
                    ctx.ident(method),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::MethodHasNoBody {
                location,
                ty,
                method,
            } => build_report(
                location,
                "MethodHasNoBody",
                format!(
                    "{} of {} has no body.",
                    ctx.ident(method),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::FunctionHasNoBody { location, function } => build_report(
                location,
                "FunctionHasNoBody",
                format!("{} has no body.", ctx.ident(function)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::VariableAlreadyDefined { location, variable } => build_report(
                location,
                "VariableAlreadyDefined",
                format!("Variable {} already defined.", ctx.ident(variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UndefinedVariable { location, variable } => build_report(
                location,
                "UndefinedVariable",
                format!("Variable {} is undefined.", ctx.ident(variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::ImmutableVariable { location, variable } => build_report(
                location,
                "ImmutableVariable",
                format!("Variable {} is immutable.", ctx.ident(variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedMember {
                location,
                member_name,
            } => build_report(
                location,
                "UnresolvedMember",
                format!("Unresolved member {}.", ctx.ident(member_name)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedTraitMethod {
                method_span,
                method_name,
                trait_name,
            } => build_report(
                method_span,
                "UnresolvedTraitMethod",
                format!(
                    "Unresolved trait method {} in trait {}.",
                    ctx.ident(method_name),
                    ctx.ident(trait_name)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::FunctionParameterMismatch {
                location,
                expected,
                found,
            } => build_report(
                location,
                "FunctionParameterMismatch",
                format!(
                    "Expected {}, but found {}.",
                    ctx.debug_type(expected),
                    ctx.debug_type(found)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::GenericParameterMismatch {
                location,
                expected,
                found,
            } => build_report(
                location,
                "GenericParameterMismatch",
                format!("Expected {}, but found {}.", expected, found),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidFunctionCall {
                location,
                method_name: _method_name,
                expected,
                found,
            } => build_report(
                location,
                "InvalidFunctionCall",
                format!("Expected {} parameters, but found {}.", expected, found),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidReturn { location, message } => {
                build_report(location, "InvalidReturn", message, ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::UnreachableExpression { location } => build_report(
                location,
                "UnreachableExpression",
                "Unreachable Expression.",
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidSelfParameter { location, message } => {
                build_report(location, "InvalidSelfParameter", ctx.ident(message), ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::TypeAlreadyDefined {
                location,
                type_name,
            } => build_report(
                location,
                "TypeAlreadyDefined",
                format!("Type {} already defined.", ctx.ident(type_name)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::MemberNotPublic {
                location,
                ty,
                field,
            } => build_report(
                location,
                "MemberNotPublic",
                format!(
                    "{} not a public member of {}.",
                    ctx.ident(field),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::ModuleNotPublic { location, module } => build_report(
                location,
                "ModuleNotPublic",
                format!("{} not a public module.", ctx.ident(module)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::TypeNotPublic { location, ty } => build_report(
                location,
                "TypeNotPublic",
                format!("{} not public.", ctx.debug_type(ty)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::IndexOutOfBounds {
                location,
                index,
                length,
            } => build_report(
                location,
                "IndexOutOfBounds",
                format!("Index {} Out Of Bounds {}.", index, length),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidCast {
                location,
                expected,
                found,
            } => build_report(
                location,
                "InvalidCast",
                format!("Expected {}, but found {}.", expected, found),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::DuplicateWildcard { location } => {
                build_report(location, "DuplicateWildcard", "Duplicate Wildcard.", ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::IncompleteMatch { location, message } => {
                build_report(location, "IncompleteMatch", message, ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::DuplicatedMethod {
                location,
                ty,
                method,
            } => build_report(
                location,
                "DuplicatedMethod",
                format!(
                    "Method {} already exists on {}.",
                    ctx.ident(method),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::NoParentModule { location } => {
                build_report(location, "NoParentModule", "No parent module.", ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::ModuleNotFound { location, module } => build_report(
                location,
                "ModuleNotFound",
                format!("Module {} not found.", ctx.ident(module)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        },
        Error::UndefinedFunction => format!("{}", error),
        Error::UncertainLoopCondition { loop_span } => build_report(
            loop_span,
            "UncertainLoopCondition",
            "Uncertain Loop Condition",
            ctx,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
    }
}
