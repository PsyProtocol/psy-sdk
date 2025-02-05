use crate::{ExprId, NodeInfo, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct StorageWriteNode {
    pub offset: ExprId,
    pub value: ExprId,
}

impl NodeInfo for StorageWriteNode {
    fn node_type(&self) -> NodeType {
        NodeType::StorageStmt
    }
}
