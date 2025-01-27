use qed_ast::{BinaryOperator, ExprId, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedBinaryNode {
    pub lhs: ExprId,
    pub operator: BinaryOperator,
    pub rhs: ExprId,
    pub type_id: TypeId,
}

impl CheckedBinaryNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::BinaryExpr
    }
}
