use qed_common::FileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub file_id: FileId,
    pub line: usize,
    pub column: usize,
}
