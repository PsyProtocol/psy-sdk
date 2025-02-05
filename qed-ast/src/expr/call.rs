use crate::{AstVisitor, DefId, ExprId, NodeInfo, NodeType, PathNode, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CallNode {
    pub variable: ExprId,
    pub receiver: Option<ExprId>,
    pub generic_parameters: Vec<UncheckedType>,
    pub args: Vec<ExprId>,
}

impl NodeInfo for CallNode {
    fn node_type(&self) -> NodeType {
        NodeType::CallExpr
    }
}
