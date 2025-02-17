use qed_ast::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum CheckedIntrinsicStmtNode {
    Assert {
        left: ExprId,
        message: Option<String>,
    },
    AssertEq {
        left: ExprId,
        right: ExprId,
        message: Option<String>,
    },
}

impl NodeInfo for CheckedIntrinsicStmtNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicStmt
    }
}
