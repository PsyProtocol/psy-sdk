use ariadne::{ColorGenerator, Label, Report, ReportKind};
use qed_ast::FileSpan;
use qed_sema::Error as SemaError;
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
    UncertainLoopCondition { loop_span: FileSpan },
    // #[error("index out of bounds")]
    // IndexOutOfBounds,
    // #[error("type mismatch")]
    // TypeMismatch,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn lowering_error_to_report(error: Error) -> Report<'static, FileSpan> {
    let mut colors = ColorGenerator::new();
    colors.next();
    match error {
        Error::ParseError(error) => panic!("{}", error),
        Error::IoError(error) => panic!("{}", error),
        Error::SemaError(error) => qed_sema::lowering_error_to_report(error),
        Error::UndefinedFunction => panic!("{}", error),
        Error::UncertainLoopCondition { loop_span } => {
            Report::build(ReportKind::Error, loop_span.clone())
                .with_code("UncertainLoopCondition")
                .with_label(
                    Label::new(loop_span.clone())
                        .with_message(format!("Uncertain Loop Condition"))
                        .with_color(colors.next()),
                )
                .with_message("UncertainLoopCondition.")
                .finish()
        }
    }
}
