use qed_ast::{ExprId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStorageReadNode {
    pub offset: ExprId,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedStorageReadNode {
    fn node_type(&self) -> NodeType {
        NodeType::StorageStmt
    }
}
