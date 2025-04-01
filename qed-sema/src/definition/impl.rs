use qed_ast::{Comment, DefId, Location, NodeInfo, NodeType};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedImplNode {
    pub generic_parameters: Vec<TypeId>,
    pub ty: TypeId,
    pub body: Vec<DefId>,
    pub scope_id: ScopeId,
    pub comments: Vec<Comment>,
    pub location: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedTraitImplNode {
    pub generic_parameters: Vec<TypeId>,
    pub trait_ty: TypeId,
    pub ty: TypeId,
    pub body: Vec<DefId>,
    pub scope_id: ScopeId,
    pub comments: Vec<Comment>,
    pub location: Location,
}

impl NodeInfo for CheckedImplNode {
    fn node_type(&self) -> NodeType {
        NodeType::ImplDef
    }
}

impl NodeInfo for CheckedTraitImplNode {
    fn node_type(&self) -> NodeType {
        NodeType::TraitImplDef
    }
}
