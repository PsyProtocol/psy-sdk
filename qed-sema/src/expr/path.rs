use qed_ast::{IdentId, NodeType};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedPathNode {
    pub name: IdentId,
    pub type_id: TypeId,
    pub scope_id: ScopeId,
}

impl CheckedPathNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
