use crate::{ExprId, NodeInfo, NodeType, Span, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CastNode {
    pub value: ExprId,
    pub target_type: UncheckedType,
    pub span: Span,
}

impl CastNode {
    pub fn new(value: ExprId, target_type: UncheckedType, span: Span) -> Self {
        Self {
            value,
            target_type,
            span,
        }
    }
}

impl NodeInfo for CastNode {
    fn node_type(&self) -> NodeType {
        NodeType::CastExpr
    }
}
