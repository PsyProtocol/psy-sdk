use crate::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnNode {
    pub expr_id: Option<ExprId>,
    pub span: Span,
}

impl NodeInfo for ReturnNode {
    fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
