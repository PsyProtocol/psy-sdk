use qed_ast::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum CheckedIntrinsicStmtNode {
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

impl NodeInfo for CheckedIntrinsicStmtNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicStmt
    }
}
