use qed_ast::{ExprId, IdentId, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedIndexAccessNode {
    pub value: ExprId,
    pub index: usize,
    pub type_id: TypeId,
}

impl CheckedIndexAccessNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::IndexAccessExpr
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedMemberAccessNode {
    pub value: ExprId,
    pub field: IdentId,
    pub type_id: TypeId,
}

impl CheckedMemberAccessNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::MemberAccessExpr
    }
}
