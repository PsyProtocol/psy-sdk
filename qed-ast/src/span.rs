use qed_common::FileId;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Span {
    pub file_id: FileId,
    pub start: usize,
    pub end: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            file_id: FileId(0),
            start: 0,
            end: 0,
        }
    }
}

impl Span {
    pub fn new(file_id: FileId, start: usize, end: usize) -> Self {
        Self {
            file_id,
            start,
            end,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSpan {
    pub path: String,
    pub start: usize,
    pub end: usize,
}

impl Default for FileSpan {
    fn default() -> Self {
        Self {
            path: String::new(),
            start: 0,
            end: 0,
        }
    }
}

impl FileSpan {
    pub fn new(path: String, start: usize, end: usize) -> Self {
        Self { path, start, end }
    }
}

impl ariadne::Span for FileSpan {
    type SourceId = String;

    fn source(&self) -> &Self::SourceId {
        &self.path
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}
