use qed_ast::{ExprId, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStorageWriteNode {
    pub offset: ExprId,
    pub value: ExprId,
}

impl CheckedStorageWriteNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::StorageStmt
    }
}
