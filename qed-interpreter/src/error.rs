use ariadne::{Label, Report, ReportKind};
use core::fmt;
use qed_ast::{Location, Program, VisitorContext};
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
    #[error("assertion failure: {message}")]
    AssertionFailure {
        message: String,
        location: Option<Location>,
    },
    // #[error("index out of bounds")]
    // IndexOutOfBounds,
    // #[error("type mismatch")]
    // TypeMismatch,
}

pub type Result<T> = std::result::Result<T, Error>;

fn build_report<F: Clone + From<u32> + ContextFelt>(
    location: Location,
    code: impl fmt::Display,
    message: impl fmt::Display,
    program: &Program<F>,
) -> Result<String> {
    let file_location = program.convert_location(&location);
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

pub fn lowering_parse_error<F: Clone + From<u32> + ContextFelt>(
    error: &qed_parser::Error,
    program: &Program<F>,
) -> String {
    match error {
        ParseError::LexicalError(error) => format!("{}", error),
        ParseError::CommonError(error) => format!("{}", error),
        ParseError::IoError(error) => format!("{}", error),
        ParseError::FileUnresolved => format!("{}", error),
        ParseError::InvalidModuleName => format!("{}", error),
        ParseError::ExternFnNotInStd => format!("{}", error),
        ParseError::FunctionBodyMissing => format!("{}", error),
        ParseError::InvalidSelfParameter => format!("{}", error),
        ParseError::InvalidToken { location } => {
            build_report(location.clone(), "InvalidToken", "Invalid Token.", program)
                .unwrap_or_else(|e| format!("Failed to build report: {}", e))
        }
        ParseError::UnrecognizedEof { expected, location } => build_report(
            location.clone(),
            "UnrecognizedEof",
            format!("Expected {:?}.", expected),
            program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        ParseError::UnrecognizedToken {
            token,
            expected,
            location,
        } => build_report(
            location.clone(),
            "UnrecognizedToken",
            format!(
                "Found unrecognized token {}, expected {:?}.",
                token, expected,
            ),
            program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        ParseError::ExtraToken { token, location } => build_report(
            location.clone(),
            "ExtraToken",
            format!("Extra token {} found.", token),
            program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
    }
}

pub fn lowering_sema_error<F: Clone + From<u32> + ContextFelt, C>(
    error: &qed_sema::Error,
    ctx: &TypeCheckerVisitorContext<F, C>,
) -> String {
    match error {
        SemaError::AnyhowError(error) => format!("{}", error),
        SemaError::CommonError(error) => format!("{}", error),
        SemaError::TypeMismatch {
            location,
            expected,
            found,
        } => build_report(
            location.clone(),
            "TypeMismatch",
            format!(
                "Expected {}, but found {}.",
                expected
                    .into_iter()
                    .map(|ty| ctx.debug_type(ty.clone()))
                    .collect::<Vec<_>>()
                    .join(","),
                ctx.debug_type(found.clone())
            ),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::InvalidPathSegment { location, segment } => build_report(
            location.clone(),
            "InvalidPathSegment",
            format!("Invalid path segment {}.", segment),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::UnresolvedType {
            location,
            resolved_type,
        } => build_report(
            location.clone(),
            "UnresolvedType",
            format!("Unresolved type {}.", ctx.ident(resolved_type.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::TraitAlreadyImplemented {
            location,
            trait_ty,
            ty,
        } => build_report(
            location.clone(),
            "TraitAlreadyImplemented",
            format!(
                "Trait {} already implemented for {}.",
                ctx.debug_type(trait_ty.clone()),
                ctx.debug_type(ty.clone())
            ),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::VariableAlreadyDefined { location, variable } => build_report(
            location.clone(),
            "VariableAlreadyDefined",
            format!("Variable {} already defined.", ctx.ident(variable.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::ImmutableVariable { location, variable } => build_report(
            location.clone(),
            "ImmutableVariable",
            format!("Variable {} is immutable.", ctx.ident(variable.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::UnresolvedMember {
            location,
            member_name,
        } => build_report(
            location.clone(),
            "UnresolvedMember",
            format!("Unresolved member {}.", ctx.ident(member_name.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::UnresolvedTraitMethod {
            method_location,
            method_name,
            trait_name,
        } => build_report(
            method_location.clone(),
            "UnresolvedTraitMethod",
            format!(
                "Unresolved trait method {} in trait {}.",
                ctx.ident(method_name.clone()),
                ctx.ident(trait_name.clone())
            ),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::InvalidGenericArguments {
            location,
            expected,
            found,
        } => build_report(
            location.clone(),
            "GenericParameterMismatch",
            format!("Expected {}, but found {}.", expected, found),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::InvalidFunctionArguments {
            location,
            method_name: _method_name,
            expected,
            found,
        } => build_report(
            location.clone(),
            "InvalidFunctionCall",
            format!("Expected {} parameters, but found {}.", expected, found),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::InvalidReturn { location, message } => {
            build_report(location.clone(), "InvalidReturn", message, &ctx.program)
                .unwrap_or_else(|e| format!("Failed to build report: {}", e))
        }
        SemaError::InvalidGenericConstraint { location } => build_report(
            location.clone(),
            "InvalidGenericConstraint",
            "Generic constraint should either be a concrete type or a list of trait requirements",
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::UnreachableExpression { location } => build_report(
            location.clone(),
            "UnreachableExpression",
            "Unreachable Expression.",
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::TypeAlreadyDefined {
            location,
            type_name,
        } => build_report(
            location.clone(),
            "TypeAlreadyDefined",
            format!("Type {} already defined.", ctx.ident(type_name.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::MemberNotPublic {
            location,
            ty,
            field,
        } => build_report(
            location.clone(),
            "MemberNotPublic",
            format!(
                "{} not a public member of {}.",
                ctx.ident(field.clone()),
                ctx.debug_type(ty.clone())
            ),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::ModuleNotPublic { location, module } => build_report(
            location.clone(),
            "ModuleNotPublic",
            format!("{} not a public module.", ctx.ident(module.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::TypeNotPublic { location, ty } => build_report(
            location.clone(),
            "TypeNotPublic",
            format!("{} not public.", ctx.debug_type(ty.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::IndexOutOfBounds {
            location,
            index,
            length,
        } => build_report(
            location.clone(),
            "IndexOutOfBounds",
            format!("Index {} Out Of Bounds {}.", index, length),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::InvalidCast {
            location,
            expected,
            found,
        } => build_report(
            location.clone(),
            "InvalidCast",
            format!("Expected {}, but found {}.", expected, found),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::DuplicateWildcard { location } => build_report(
            location.clone(),
            "DuplicateWildcard",
            "Duplicate Wildcard.",
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::IncompleteMatch { location, message } => {
            build_report(location.clone(), "IncompleteMatch", message, &ctx.program)
                .unwrap_or_else(|e| format!("Failed to build report: {}", e))
        }
        SemaError::NoParentModule { location } => build_report(
            location.clone(),
            "NoParentModule",
            "No parent module.",
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::ModuleNotFound { location, module } => build_report(
            location.clone(),
            "ModuleNotFound",
            format!("Module {} not found.", ctx.ident(module.clone())),
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        SemaError::SpecializationNotAllowed { location } => build_report(
            location.clone(),
            "SpecializationNotAllowed",
            "Specialization not allowed.",
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
    }
}

pub fn lowering_interpreter_error<F: Clone + From<u32> + ContextFelt, C>(
    error: Error,
    ctx: &TypeCheckerVisitorContext<F, C>,
) -> anyhow::Error {
    let context = match &error {
        Error::ParseError(error) => lowering_parse_error(error, &ctx.program),
        Error::IoError(error) => format!("{}", error),
        Error::SemaError(error) => lowering_sema_error(error, ctx),
        Error::UndefinedFunction => format!("{}", error),
        Error::UncertainLoopCondition { loop_location } => build_report(
            loop_location.clone(),
            "UncertainLoopCondition",
            "Uncertain Loop Condition",
            &ctx.program,
        )
        .unwrap_or_else(|e| format!("Failed to build report: {}", e)),
        Error::AssertionFailure { message, location } => {
            if let Some(location) = location {
                build_report(location.clone(), "AssertionFailure", message, &ctx.program)
                    .unwrap_or_else(|e| format!("Failed to build report: {}", e))
            } else {
                format!("Assertion failure: {}", message)
            }
        }
    };

    anyhow::Error::from(error).context(context)
}
