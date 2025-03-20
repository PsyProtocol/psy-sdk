use crate::{ExprId, Location, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum IntrinsicStmtNode {
    Assert {
        left: ExprId,
        message: Option<String>,
        location: Location,
    },
    AssertEq {
        left: ExprId,
        right: ExprId,
        message: Option<String>,
        location: Location,
    },
}

impl NodeInfo for IntrinsicStmtNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicStmt
    }
}
