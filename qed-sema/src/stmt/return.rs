use qed_ast::{ExprId, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedReturnNode {
    pub ret: Option<(ExprId, TypeId)>,
}

impl CheckedReturnNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
