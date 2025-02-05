use crate::{ExprId, NodeInfo, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct StorageReadNode {
    pub offset: ExprId,
}

impl NodeInfo for StorageReadNode {
    fn node_type(&self) -> NodeType {
        NodeType::StorageExpr
    }
}
