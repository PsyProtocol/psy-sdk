use crate::{AstVisitor, ExprId, IdentId, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct IndexAccessNode {
    pub value: ExprId,
    pub index: usize,
}

impl IndexAccessNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::IndexAccessExpr
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccessNode {
    pub value: ExprId,
    pub field: IdentId,
}

impl MemberAccessNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::MemberAccessExpr
    }
}
