use qed_ast::{ExprId, IdentId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedIndexAccessNode {
    pub value: ExprId,
    pub index: usize,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedIndexAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::IndexAccessExpr
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedMemberAccessNode {
    pub value: ExprId,
    pub field: IdentId,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedMemberAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::MemberAccessExpr
    }
}
