use ariadne::{Label, Report, ReportKind};
use core::fmt;
use qed_ast::{Location, VisitorContext};
use qed_parser::Error as ParseError;
use qed_sema::{AstVisualizer, Error as SemaError, TypeCheckerVisitorContext};
use qedlang_core::dpn::ops::context_trait::ContextFelt;
use std::io::Error as IoError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("parse error: {0}")]
    ParseError(#[from] ParseError),
    #[error("io error: {0}")]
    IoError(#[from] IoError),
    #[error("sema error: {0}")]
    SemaError(#[from] SemaError),
    #[error("undefined function")]
    UndefinedFunction,
    #[error("uncertain loop condition")]
    UncertainLoopCondition { loop_location: Location },
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
    let file_location = ctx.program.convert_location(&location);
    let report = Report::build(ReportKind::Error, file_location.clone())
        .with_code(code)
        .with_label(Label::new(file_location).with_message(message))
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
        Error::ParseError(error) => match error {
            ParseError::LexicalError(error) => format!("{}", error),
            ParseError::CommonError(error) => format!("{}", error),
            ParseError::IoError(error) => format!("{}", error),
            ParseError::FileUnresolved => format!("{}", error),
            ParseError::InvalidModuleName => format!("{}", error),
            ParseError::ExternFnNotInStd => format!("{}", error),
            ParseError::FunctionBodyMissing => format!("{}", error),
            ParseError::InvalidSelfParameter => format!("{}", error),
            ParseError::InvalidToken { location } => {
                build_report(location, "InvalidToken", "Invalid Token.", ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            ParseError::UnrecognizedEof { expected, location } => build_report(
                location,
                "UnrecognizedEof",
                format!("Expected {:?}.", expected),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            ParseError::UnrecognizedToken {
                token,
                expected,
                location,
            } => build_report(
                location,
                "UnrecognizedToken",
                format!(
                    "Found unrecognized token {}, expected {:?}.",
                    token, expected,
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            ParseError::ExtraToken { token, location } => build_report(
                location,
                "ExtraToken",
                format!("Extra token {} found.", token),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        },
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
            SemaError::InvalidPathSegment { location, segment } => build_report(
                location,
                "InvalidPathSegment",
                format!("Invalid path segment {}.", ctx.ident(segment)),
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
            SemaError::VariableAlreadyDefined { location, variable } => build_report(
                location,
                "VariableAlreadyDefined",
                format!("Variable {} already defined.", ctx.ident(variable)),
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
                method_location,
                method_name,
                trait_name,
            } => build_report(
                method_location,
                "UnresolvedTraitMethod",
                format!(
                    "Unresolved trait method {} in trait {}.",
                    ctx.ident(method_name),
                    ctx.ident(trait_name)
                ),
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
            SemaError::InvalidGenericArguments {
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
            SemaError::InvalidFunctionArguments {
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
            SemaError::InvalidGenericConstraint { location } => {
                build_report(location, "InvalidGenericConstraint", "Generic constraint should either be a concrete type or a list of trait requirements", ctx)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            }
            SemaError::UnreachableExpression { location } => build_report(
                location,
                "UnreachableExpression",
                "Unreachable Expression.",
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
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
            SemaError::SpecializationNotAllowed { location } => build_report(
                location,
                "SpecializationNotAllowed",
                "Specialization not allowed.",
                ctx,
            )
            .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        },
        Error::UndefinedFunction => format!("{}", error),
        Error::UncertainLoopCondition { loop_location } => build_report(
            loop_location,
            "UncertainLoopCondition",
            "Uncertain Loop Condition",
            ctx,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
    }
}
