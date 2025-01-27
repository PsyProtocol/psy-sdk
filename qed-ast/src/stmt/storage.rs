use crate::{ExprId, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct StorageWriteNode {
    pub offset: ExprId,
    pub value: ExprId,
}

impl StorageWriteNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::StorageStmt
    }
}
