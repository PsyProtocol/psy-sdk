use qed_ast::{IdentId, NodeInfo, NodeType, Span};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedPathNode {
    pub name: IdentId,
    pub type_id: TypeId,
    pub scope_id: ScopeId,
    pub span: Span,
}

impl NodeInfo for CheckedPathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
