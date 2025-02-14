use crate::{AstVisitor, DefId, ExprId, IdentId, NodeInfo, NodeType};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct IndexAccessNode {
    pub target: ExprId,
    pub index: usize,
}

impl NodeInfo for IndexAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::IndexAccessExpr
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccessNode {
    pub target: ExprId,
    pub field: IdentId,
}

impl NodeInfo for MemberAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::MemberAccessExpr
    }
}

impl Display for MemberAccessNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberAccessNode target:{:?}, field:{}",
            self.target, self.field
        )
    }
}
