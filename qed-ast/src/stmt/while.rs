use crate::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct WhileNode {
    pub predicate: ExprId,
    pub body: ExprId,
}

impl NodeInfo for WhileNode {
    fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
