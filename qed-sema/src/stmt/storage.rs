use qed_ast::{ExprId, NodeInfo, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStorageWriteNode {
    pub offset: ExprId,
    pub value: ExprId,
}

impl NodeInfo for CheckedStorageWriteNode {
    fn node_type(&self) -> NodeType {
        NodeType::StorageStmt
    }
}
