use crate::{AstVisitor, DefId, ExprId, IdentId, NodeInfo, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct IndexAccessNode {
    pub value: ExprId,
    pub index: usize,
}

impl NodeInfo for IndexAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::IndexAccessExpr
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccessNode {
    pub value: ExprId,
    pub field: IdentId,
}

impl NodeInfo for MemberAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::MemberAccessExpr
    }
}
