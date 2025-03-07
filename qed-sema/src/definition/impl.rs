use qed_ast::{DefId, NodeInfo, NodeType};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedImplNode {
    pub generic_parameters: Vec<TypeId>,
    pub ty: TypeId,
    pub body: Vec<DefId>,
    pub scope_id: ScopeId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedImplTraitNode {
    pub generic_parameters: Vec<TypeId>,
    pub trait_ty: TypeId,
    pub ty: TypeId,
    pub body: Vec<DefId>,
    pub scope_id: ScopeId,
}

impl NodeInfo for CheckedImplNode {
    fn node_type(&self) -> NodeType {
        NodeType::ImplDef
    }
}

impl NodeInfo for CheckedImplTraitNode {
    fn node_type(&self) -> NodeType {
        NodeType::ImplTraitDef
    }
}
