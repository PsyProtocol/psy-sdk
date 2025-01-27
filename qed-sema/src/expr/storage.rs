use qed_ast::{ExprId, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStorageReadNode {
    pub offset: ExprId,
    pub type_id: TypeId,
}

impl CheckedStorageReadNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::StorageStmt
    }
}
