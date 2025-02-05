use crate::{AstVisitor, ExprId, NodeInfo, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CastNode {
    pub value: ExprId,
    pub target_type: UncheckedType,
}

impl CastNode {
    pub fn new(value: ExprId, target_type: UncheckedType) -> Self {
        Self { value, target_type }
    }
}

impl NodeInfo for CastNode {
    fn node_type(&self) -> NodeType {
        NodeType::CastExpr
    }
}
