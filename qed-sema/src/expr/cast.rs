use qed_ast::NodeType;

use crate::{ExprId, TypeId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCastNode {
    pub value: ExprId,
    pub target_type: TypeId,
}

impl CheckedCastNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::CastExpr
    }
}
