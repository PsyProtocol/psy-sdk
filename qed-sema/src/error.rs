use ariadne::{ColorGenerator, Label, Report, ReportKind};
use qed_ast::FileSpan;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    AnyError(#[from] anyhow::Error),
    #[error("type mismatch")]
    TypeMismatch {
        span: FileSpan,
        expected: String,
        found: String,
    },
    #[error("unresolved path")]
    UnresolvedPath {
        span: FileSpan,
        resolved_path: String,
    },
    #[error("unresolved value")]
    UnresolvedVariable {
        span: FileSpan,
        resolved_variable: String,
    },
    #[error("unresolved type")]
    UnresolvedType {
        span: FileSpan,
        resolved_type: String,
    },
    #[error("unresolved use")]
    UnresolvedUse {
        span: FileSpan,
        resolved_use: String,
    },
    #[error("variable already defined")]
    VariableAlreadyDefined { span: FileSpan, variable: String },
    #[error("undefined variable")]
    UndefinedVariable { span: FileSpan, variable: String },
    #[error("immutable variable")]
    ImmutableVariable { span: FileSpan, variable: String },
    #[error("unresolved implementor")]
    UnresolvedImplementor {
        span: FileSpan,
        resolved_implementor: String,
    },
    #[error("unresolved trait")]
    UnresolvedTrait { span: FileSpan, trait_name: String },
    #[error("unresolved member")]
    UnresolvedMember { span: FileSpan, member_name: String },
    #[error("unresolved trait method")]
    UnresolvedTraitMethod {
        method_span: FileSpan,
        method_name: String,
        trait_name: String,
    },
    #[error("function parameter mismatch")]
    FunctionParameterMismatch {
        span: FileSpan,
        expected: String,
        found: String,
    },
    #[error("generic parameter mismatch")]
    GenericParameterMismatch {
        span: FileSpan,
        expected: String,
        found: String,
    },
    #[error("invalid function call")]
    InvalidFunctionCall {
        span: FileSpan,
        method_name: String,
        expected: String,
        found: String,
    },
    #[error("invalid return")]
    InvalidReturn { span: FileSpan, message: String },
    #[error("unreachable expression")]
    UnreachableExpression { span: FileSpan },
    #[error("invalid self parameter")]
    InvalidSelfParameter { span: FileSpan, message: String },
    #[error("type already defined")]
    TypeAlreadyDefined { span: FileSpan, type_name: String },
    #[error("index out of bounds")]
    IndexOutOfBounds {
        span: FileSpan,
        index: usize,
        length: usize,
    },
    #[error("invalid cast")]
    InvalidCast {
        span: FileSpan,
        expected: String,
        found: String,
    },
    #[error("unreachable match")]
    UnreachableMatch,
    #[error("unreachable code")]
    DuplicateWildcard,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn lowering_error_to_report(error: Error) -> Report<'static, FileSpan> {
    let mut colors = ColorGenerator::new();
    colors.next();
    match error {
        Error::AnyError(_error) => todo!(),
        Error::TypeMismatch {
            span,
            expected,
            found,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("TypeMismatch")
            .with_label(
                Label::new(span)
                    .with_message(format!("Expected {}, but founded {}.", expected, found))
                    .with_color(colors.next()),
            )
            .with_message("Type Mismatch.")
            .finish(),
        Error::UnresolvedPath {
            span,
            resolved_path,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("UnresolvedPath")
            .with_label(
                Label::new(span)
                    .with_message(format!("Unresolved path {}.", resolved_path))
                    .with_color(colors.next()),
            )
            .finish(),
        Error::UnresolvedVariable {
            span,
            resolved_variable,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("UnresolvedVariable")
            .with_label(
                Label::new(span)
                    .with_message(format!("Unresolved variable {}.", resolved_variable))
                    .with_color(colors.next()),
            )
            .with_message("Unresolved Variable.")
            .finish(),
        Error::UnresolvedType {
            span,
            resolved_type,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("UnresolvedType")
            .with_label(
                Label::new(span)
                    .with_message(format!("Unresolved type {}.", resolved_type))
                    .with_color(colors.next()),
            )
            .with_message("Unresolved Type.")
            .finish(),
        Error::UnresolvedUse { span, resolved_use } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("UnresolvedUse")
                .with_label(
                    Label::new(span).with_message(format!("Unresolved use {}.", resolved_use)),
                )
                .with_message("Unresolved Use.")
                .finish()
        }
        Error::VariableAlreadyDefined { span, variable } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("VariableAlreadyDefined")
                .with_label(
                    Label::new(span)
                        .with_message(format!("Variable {} already defined.", variable))
                        .with_color(colors.next()),
                )
                .with_message("Variable Already Defined.")
                .finish()
        }
        Error::UndefinedVariable { span, variable } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("UndefinedVariable")
                .with_label(
                    Label::new(span)
                        .with_message(format!("Variable {} is undefined.", variable))
                        .with_color(colors.next()),
                )
                .with_message("Undefined Variable.")
                .finish()
        }
        Error::ImmutableVariable { span, variable } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("ImmutableVariable")
                .with_label(
                    Label::new(span)
                        .with_message(format!("Variable {} is immutable.", variable))
                        .with_color(colors.next()),
                )
                .with_message("Immutable Variable.")
                .finish()
        }
        Error::UnresolvedImplementor {
            span,
            resolved_implementor,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("UnresolvedImplementor")
            .with_label(
                Label::new(span)
                    .with_message(format!("Unresolved implementor {}.", resolved_implementor))
                    .with_color(colors.next()),
            )
            .with_message("Unresolved Implementor.")
            .finish(),
        Error::UnresolvedTrait { span, trait_name } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("UnresolvedTrait")
                .with_label(
                    Label::new(span)
                        .with_message(format!("Unresolved trait {}.", trait_name))
                        .with_color(colors.next()),
                )
                .with_message("Unresolved Trait.")
                .finish()
        }
        Error::UnresolvedMember { span, member_name } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("UnresolvedMember")
                .with_label(
                    Label::new(span)
                        .with_message(format!("Unresolved member {}", member_name))
                        .with_color(colors.next()),
                )
                .with_message("Unresolved Member.")
                .finish()
        }
        Error::UnresolvedTraitMethod {
            method_span,
            method_name,
            trait_name,
        } => Report::build(ReportKind::Error, method_span.clone())
            .with_code("UnresolvedTraitMethod")
            .with_label(
                Label::new(method_span)
                    .with_message(format!(
                        "Unresolved trait method {} in trait {}.",
                        method_name, trait_name
                    ))
                    .with_color(colors.next()),
            )
            .with_message("Unresolved Trait Method.")
            .finish(),
        Error::FunctionParameterMismatch {
            span,
            expected,
            found,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("FunctionParameterMismatch")
            .with_label(
                Label::new(span)
                    .with_message(format!("Expected {}, but founded {}.", expected, found))
                    .with_color(colors.next()),
            )
            .with_message("FunctionParameterMismatch.")
            .finish(),
        Error::GenericParameterMismatch {
            span,
            expected,
            found,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("GenericParameterMismatch")
            .with_label(
                Label::new(span)
                    .with_message(format!("Expected {}, but founded {}.", expected, found))
                    .with_color(colors.next()),
            )
            .with_message("GenericParameterMismatch.")
            .finish(),
        Error::InvalidFunctionCall {
            span,
            method_name: _method_name,
            expected,
            found,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("InvalidFunctionCall")
            .with_label(
                Label::new(span)
                    .with_message(format!(
                        "Expected {} parameters, but founded {}.",
                        expected, found
                    ))
                    .with_color(colors.next()),
            )
            .with_message("InvalidFunctionCall.")
            .finish(),
        Error::InvalidReturn { span, message } => Report::build(ReportKind::Error, span.clone())
            .with_code("InvalidReturn")
            .with_label(
                Label::new(span)
                    .with_message(message)
                    .with_color(colors.next()),
            )
            .with_message("InvalidReturn.")
            .finish(),
        Error::UnreachableExpression { span } => Report::build(ReportKind::Error, span.clone())
            .with_code("UnreachableExpression")
            .with_label(
                Label::new(span)
                    .with_message("Unreachable Expression.")
                    .with_color(colors.next()),
            )
            .with_message("Unreachable Expression.")
            .finish(),
        Error::InvalidSelfParameter { span, message } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("InvalidSelfParameter")
                .with_label(
                    Label::new(span)
                        .with_message(message)
                        .with_color(colors.next()),
                )
                .with_message("Unresolved import.")
                .finish()
        }
        Error::TypeAlreadyDefined { span, type_name } => {
            Report::build(ReportKind::Error, span.clone())
                .with_code("TypeAlreadyDefined")
                .with_label(
                    Label::new(span)
                        .with_message(format!("Type {} already defined.", type_name))
                        .with_color(colors.next()),
                )
                .with_message("TypeAlreadyDefined.")
                .finish()
        }
        Error::IndexOutOfBounds {
            span,
            index,
            length,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("IndexOutOfBounds")
            .with_label(
                Label::new(span)
                    .with_message(format!("Index {} Out Of Bounds {}", index, length))
                    .with_color(colors.next()),
            )
            .with_message("IndexOutOfBounds.")
            .finish(),
        Error::InvalidCast {
            span,
            expected,
            found,
        } => Report::build(ReportKind::Error, span.clone())
            .with_code("InvalidCast")
            .with_label(
                Label::new(span)
                    .with_message(format!("Expected {}, but founded {}.", expected, found))
                    .with_color(colors.next()),
            )
            .with_message("InvalidCast.")
            .finish(),
        Error::DuplicateWildcard { .. } => todo!(),
        Error::UnreachableMatch { .. } => todo!(),
    }
}
