use crate::{AstVisitor, DefId, ExprId, NodeInfo, NodeType, PathNode, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct AssertNode {
    pub left: ExprId,
    pub message: Option<String>,
}
impl NodeInfo for AssertNode {
    fn node_type(&self) -> NodeType {
        NodeType::AssertStmt
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssertEqNode {
    pub left: ExprId,
    pub right: ExprId,
    pub message: Option<String>,
}
impl NodeInfo for AssertEqNode {
    fn node_type(&self) -> NodeType {
        NodeType::AssertEqStmt
    }
}
