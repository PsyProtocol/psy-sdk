use crate::{AstVisitor, ExprId, NodeType, PathNode, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CallNode {
    pub variable: ExprId,
    pub receiver: Option<ExprId>,
    pub generic_parameters: Vec<UncheckedType>,
    pub args: Vec<ExprId>,
}

impl CallNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::CallExpr
    }
}
