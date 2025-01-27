use crate::{ExprId, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct StorageReadNode {
    pub offset: ExprId,
}

impl StorageReadNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::StorageExpr
    }
}
