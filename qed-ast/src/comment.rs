use crate::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comment {
    Line { content: String, location: Location },
    Block { content: String, location: Location },
}

impl Comment {
    pub fn new_line(content: String, location: Location) -> Self {
        Self::Line { content, location }
    }

    pub fn new_block(content: String, location: Location) -> Self {
        Self::Block { content, location }
    }
}
