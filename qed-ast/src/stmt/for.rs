use crate::{ExprId, Identifier, Location, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct ForNode {
    pub variable: Identifier,
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
