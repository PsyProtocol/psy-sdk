use qed_ast::{ExprId, NodeInfo, NodeType};

use crate::{CheckedBlockNode, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedWhileNode {
    pub predicate: ExprId,
    pub type_id: TypeId,
    pub body: CheckedBlockNode,
}

impl NodeInfo for CheckedWhileNode {
    fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
