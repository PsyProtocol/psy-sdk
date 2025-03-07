use qed_ast::{NodeInfo, NodeType, Span};

use crate::{ExprId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCastNode {
    pub value: ExprId,
    pub target_type: TypeId,
    pub span: Span,
}

impl NodeInfo for CheckedCastNode {
    fn node_type(&self) -> NodeType {
        NodeType::CastExpr
    }
}
