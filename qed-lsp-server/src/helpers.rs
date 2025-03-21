use std::collections::HashMap;


use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, Location, Position, PublishDiagnosticsParams, Range,
    TextDocumentItem, Url,
};


use unicode_segmentation::UnicodeSegmentation;

/// Returns a string from a range of human characters (graphemes). Respects unicode.
pub fn str_range(s: &str, range: &std::ops::Range<usize>) -> String {
    s.graphemes(true)
        .skip(range.start)
        .take(range.len())
        .collect()
}
