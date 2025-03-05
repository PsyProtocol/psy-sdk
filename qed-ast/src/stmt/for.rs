use crate::{ExprId, IdentId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct ForNode {
    pub variable: IdentId,
    pub start: ExprId,
    pub end: ExprId,
    pub body: ExprId,
}

impl NodeInfo for ForNode {
    fn node_type(&self) -> NodeType {
        NodeType::ForStmt
    }
}
