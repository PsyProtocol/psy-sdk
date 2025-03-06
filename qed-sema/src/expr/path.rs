use qed_ast::{IdentId, NodeInfo, NodeType};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedPathNode {
    pub root: Option<TypeId>,
    pub target: IdentId,
    pub type_id: TypeId,
    pub scope_id: ScopeId,
}

impl NodeInfo for CheckedPathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
