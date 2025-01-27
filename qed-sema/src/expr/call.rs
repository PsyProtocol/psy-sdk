use qed_ast::{ExprId, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCallNode {
    pub variable: ExprId,
    pub receiver: Option<ExprId>,
    pub generic_parameters: Vec<TypeId>,
    pub args: Vec<ExprId>,
    pub type_id: TypeId,
}

impl CheckedCallNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::CallExpr
    }
}
