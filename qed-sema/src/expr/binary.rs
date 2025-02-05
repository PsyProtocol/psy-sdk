use qed_ast::{BinaryOperator, ExprId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedBinaryNode {
    pub lhs: ExprId,
    pub operator: BinaryOperator,
    pub rhs: ExprId,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedBinaryNode {
    fn node_type(&self) -> NodeType {
        NodeType::BinaryExpr
    }
}
