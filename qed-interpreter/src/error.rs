use ariadne::{ColorGenerator, Label, Report, ReportKind};
use core::fmt;
use qed_ast::{FileSpan, Span, VisitorContext};
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
    UncertainLoopCondition { loop_span: Span },
    // #[error("index out of bounds")]
    // IndexOutOfBounds,
    // #[error("type mismatch")]
    // TypeMismatch,
}

pub type Result<T> = std::result::Result<T, Error>;

fn build_report<F: Clone + From<u32> + ContextFelt, C>(
    span: Span,
    code: impl fmt::Display,
    message: impl fmt::Display,
    ctx: &TypeCheckerVisitorContext<F, C>,
) -> Result<String> {
    let file_span = ctx.program.convert_span(&span);
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
                span,
                expected,
                found,
            } => build_report(
                span,
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
                span,
                resolved_path,
            } => build_report(
                span,
                "UnresolvedPath",
                format!("Unresolved path {}.", resolved_path),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidPathSegment { span, segment } => build_report(
                span,
                "InvalidPathSegment",
                format!("Invalid path segment {}.", ctx.ident(segment)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedVariable {
                span,
                resolved_variable,
            } => build_report(
                span,
                "UnresolvedVariable",
                format!("Unresolved variable {}.", ctx.ident(resolved_variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedType {
                span,
                resolved_type,
            } => build_report(
                span,
                "UnresolvedType",
                format!("Unresolved type {}.", ctx.ident(resolved_type)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::TraitAlreadyImplemented { span, trait_ty, ty } => build_report(
                span,
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
                span,
                trait_ty,
                ty,
                method,
            } => build_report(
                span,
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
            SemaError::MethodHasNoBody { span, ty, method } => build_report(
                span,
                "MethodHasNoBody",
                format!(
                    "{} of {} has no body.",
                    ctx.ident(method),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::FunctionHasNoBody { span, function } => build_report(
                span,
                "FunctionHasNoBody",
                format!("{} has no body.", ctx.ident(function)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::VariableAlreadyDefined { span, variable } => build_report(
                span,
                "VariableAlreadyDefined",
                format!("Variable {} already defined.", ctx.ident(variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UndefinedVariable { span, variable } => build_report(
                span,
                "UndefinedVariable",
                format!("Variable {} is undefined.", ctx.ident(variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::ImmutableVariable { span, variable } => build_report(
                span,
                "ImmutableVariable",
                format!("Variable {} is immutable.", ctx.ident(variable)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::UnresolvedMember { span, member_name } => build_report(
                span,
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
                span,
                expected,
                found,
            } => build_report(
                span,
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
                span,
                expected,
                found,
            } => build_report(
                span,
                "GenericParameterMismatch",
                format!("Expected {}, but found {}.", expected, found),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidFunctionCall {
                span,
                method_name: _method_name,
                expected,
                found,
            } => build_report(
                span,
                "InvalidFunctionCall",
                format!("Expected {} parameters, but found {}.", expected, found),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidReturn { span, message } => {
                build_report(span, "InvalidReturn", message, ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::UnreachableExpression { span } => build_report(
                span,
                "UnreachableExpression",
                "Unreachable Expression.",
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidSelfParameter { span, message } => {
                build_report(span, "InvalidSelfParameter", ctx.ident(message), ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::TypeAlreadyDefined { span, type_name } => build_report(
                span,
                "TypeAlreadyDefined",
                format!("Type {} already defined.", ctx.ident(type_name)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::MemberNotPublic { span, ty, field } => build_report(
                span,
                "MemberNotPublic",
                format!(
                    "{} not a public member of {}.",
                    ctx.ident(field),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::ModuleNotPublic { span, module } => build_report(
                span,
                "ModuleNotPublic",
                format!("{} not a public module.", ctx.ident(module)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::TypeNotPublic { span, ty } => build_report(
                span,
                "TypeNotPublic",
                format!("{} not public.", ctx.debug_type(ty)),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::IndexOutOfBounds {
                span,
                index,
                length,
            } => build_report(
                span,
                "IndexOutOfBounds",
                format!("Index {} Out Of Bounds {}.", index, length),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidCast {
                span,
                expected,
                found,
            } => build_report(
                span,
                "InvalidCast",
                format!("Expected {}, but found {}.", expected, found),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::DuplicateWildcard { span } => {
                build_report(span, "DuplicateWildcard", "Duplicate Wildcard.", ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::IncompleteMatch { span, message } => {
                build_report(span, "IncompleteMatch", message, ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::DuplicatedMethod { span, ty, method } => build_report(
                span,
                "DuplicatedMethod",
                format!(
                    "Method {} already exists on {}.",
                    ctx.ident(method),
                    ctx.debug_type(ty)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::NoParentModule { span } => {
                build_report(span, "NoParentModule", "No parent module.", ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::ModuleNotFound { span, module } => build_report(
                span,
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
