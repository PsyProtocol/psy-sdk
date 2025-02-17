use crate::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum IntrinsicStmtNode {
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

impl NodeInfo for IntrinsicStmtNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicStmt
    }
}
