use crate::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct WhileNode {
    pub predicate: ExprId,
    pub body: ExprId,
    pub span: Span,
}

impl NodeInfo for WhileNode {
    fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
