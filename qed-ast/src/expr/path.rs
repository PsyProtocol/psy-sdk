use crate::{AstVisitor, DefId, ExprId, IdentId, NodeInfo, NodeType};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    pub root: Option<IdentId>,
    pub segments: Vec<IdentId>,
    pub target: IdentId,
}

impl PathNode {
    pub fn new(root: Option<IdentId>, target: IdentId, segments: Vec<IdentId>) -> Self {
        Self {
            root,
            segments,
            target,
        }
    }
}

impl NodeInfo for PathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}

impl Display for PathNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PathNode: ")?;
        match &self.root {
            Some(root) => write!(f, "IdentId({})", root),
            None => write!(f, "NO_ROOT"),
        }?;
        for seg in &self.segments {
            write!(f, "::{}", seg)?;
        }
        writeln!(f, "::{}", self.target)
    }
}
