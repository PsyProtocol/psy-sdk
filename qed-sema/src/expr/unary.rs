use qed_ast::{ExprId, NodeType, UnaryOperator};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedUnaryNode {
    pub operator: UnaryOperator,
    pub rhs: ExprId,
    pub type_id: TypeId,
}

impl CheckedUnaryNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::UnaryExpr
    }
}
