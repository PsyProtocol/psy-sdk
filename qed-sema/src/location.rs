use qed_ast::{Location, Position};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::TypeCheckerVisitorContext;

impl<F: Clone + From<u32> + ContextFelt, C> TypeCheckerVisitorContext<F, C> {
    pub fn location_to_position(&self, location: Location) -> Option<(Position, Position)> {
        let file_content = self
            .program
            .file_resolver
            .resolve_content(&location.file_id)?;
        let (start_line, start_column) = line_and_column_from_offset(&file_content, location.start);
        let (end_line, end_column) = line_and_column_from_offset(&file_content, location.end);
        Some((
            Position {
                file_id: location.file_id,
                line: start_line,
                column: start_column,
            },
            Position {
                file_id: location.file_id,
                line: end_line,
                column: end_column,
            },
        ))
    }

    pub fn position_to_location(&self, position: Position) -> Option<Location> {
        let file_content = self
            .program
            .file_resolver
            .resolve_content(&position.file_id)?;
        let offset = offset_from_position(&file_content, &position);
        Some(Location {
            file_id: position.file_id,
            start: offset,
            end: offset + 1,
        })
    }

    pub fn position_to_file_path(&self, position: Position) -> Option<String> {
        format!(
            "{}:{}:{}",
            self.program
                .file_resolver
                .resolve_path(&position.file_id)?
                .display(),
            position.line,
            position.column
        )
        .into()
    }
}

pub fn line_and_column_from_offset(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 0;

    for (i, char) in source.chars().enumerate() {
        column += 1;

        if char == '\n' {
            line += 1;
            column = 0;
        }

        if offset <= i {
            break;
        }
    }

    (line, column)
}

pub fn offset_from_position(source: &str, position: &Position) -> usize {
    let mut offset = 0;
    let lines = source.lines().collect::<Vec<&str>>();

    // Check if the line number is out of line
    if position.line as usize >= lines.len() {
        return source.len();
    }

    for i in 0..position.line as usize {
        offset += lines[i].len() + 1; // +1 for '\n'
    }

    let current_line = lines[position.line as usize];
    if position.column as usize >= current_line.len() {
        offset += current_line.len();
    } else {
        offset += position.column;
    }

    offset
}
