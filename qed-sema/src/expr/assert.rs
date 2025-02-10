use crate::TypeId;
use qed_ast::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedAssertNode {
    pub left: ExprId,
    pub message: Option<String>,
}

impl NodeInfo for CheckedAssertNode {
    fn node_type(&self) -> NodeType {
        NodeType::AssertExpr
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedAssertEqNode {
    pub left: ExprId,
    pub right: ExprId,
    pub message: Option<String>,
}

impl NodeInfo for CheckedAssertEqNode {
    fn node_type(&self) -> NodeType {
        NodeType::AssertEqExpr
    }
}
