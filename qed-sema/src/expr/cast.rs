use qed_ast::{NodeInfo, NodeType};

use crate::{ExprId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCastNode {
    pub value: ExprId,
    pub target_type: TypeId,
}

impl NodeInfo for CheckedCastNode {
    fn node_type(&self) -> NodeType {
        NodeType::CastExpr
    }
}
