use crate::{ExprId, IdentId, Location, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct ForNode {
    pub variable: IdentId,
    pub start: ExprId,
    pub end: ExprId,
    pub body: ExprId,
    pub location: Location,
}

impl NodeInfo for ForNode {
    fn node_type(&self) -> NodeType {
        NodeType::ForStmt
    }
}
