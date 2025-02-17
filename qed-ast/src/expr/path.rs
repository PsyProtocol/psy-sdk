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

    pub fn is_receiver(&self) -> bool {
        self.root.is_none() && self.target == IdentId::SELF && self.segments.is_empty()
    }
}

impl NodeInfo for PathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
