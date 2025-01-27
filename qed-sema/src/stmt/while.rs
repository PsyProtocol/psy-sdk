use qed_ast::{ExprId, NodeType};

use crate::{CheckedBlockNode, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedWhileNode {
    pub predicate: ExprId,
    pub type_id: TypeId,
    pub body: CheckedBlockNode,
}

impl CheckedWhileNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
