use crate::{ExprId, IdentId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct ForNode {
    pub variable: IdentId,
    pub start: ExprId,
    pub end: ExprId,
    pub body: ExprId,
    pub span: Span,
}

impl NodeInfo for ForNode {
    fn node_type(&self) -> NodeType {
        NodeType::ForStmt
    }
}
