use qed_ast::{DefId, IdentId, NodeInfo, NodeType, Span, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedTraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub body: Vec<DefId>,
    pub unchecked_body: Vec<DefId>,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
    pub span: Span,
}

impl NodeInfo for CheckedTraitNode {
    fn node_type(&self) -> NodeType {
        NodeType::TraitDef
    }
}
