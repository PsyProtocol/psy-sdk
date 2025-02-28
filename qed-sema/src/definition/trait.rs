use qed_ast::{DefId, IdentId, NodeInfo, NodeType, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedTraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub body: Vec<DefId>,
    pub def_ids: Vec<DefId>,
    pub implementors: Vec<TypeId>,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
}

impl CheckedTraitNode {
    pub fn add_implementor(&mut self, implementor: TypeId) {
        self.implementors.push(implementor);
    }
}

impl NodeInfo for CheckedTraitNode {
    fn node_type(&self) -> NodeType {
        NodeType::TraitDef
    }
}
