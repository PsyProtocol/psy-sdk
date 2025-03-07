use crate::{ScopeId, TypeId};
use qed_ast::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedMatchNode {
    pub value: ExprId,
    pub cases: Vec<CheckedMatchArm>,
    pub type_id: TypeId,
    pub scope_id: ScopeId,
}

impl NodeInfo for CheckedMatchNode {
    fn node_type(&self) -> NodeType {
        NodeType::MatchExpr
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedMatchArm {
    pub pattern: Option<ExprId>,
    pub body: ExprId,
}
