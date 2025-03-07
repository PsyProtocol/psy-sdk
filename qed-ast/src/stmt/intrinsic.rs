use crate::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum IntrinsicStmtNode {
    Assert {
        left: ExprId,
        message: Option<String>,
        span: Span,
    },
    AssertEq {
        left: ExprId,
        right: ExprId,
        message: Option<String>,
        span: Span,
    },
}

impl NodeInfo for IntrinsicStmtNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicStmt
    }
}
