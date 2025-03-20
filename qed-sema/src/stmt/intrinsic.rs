use qed_ast::{ExprId, Location, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum CheckedIntrinsicStmtNode {
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

impl NodeInfo for CheckedIntrinsicStmtNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicStmt
    }
}
