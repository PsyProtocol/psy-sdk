use crate::{DefId, GenericParameter, Identifier, Location, NodeInfo, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<GenericParameter>,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
    pub location: Location,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplTraitNode {
    pub generic_parameters: Vec<GenericParameter>,
    pub trait_ty: UncheckedType,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
    pub location: Location,
}

impl NodeInfo for ImplNode {
    fn node_type(&self) -> NodeType {
        NodeType::ImplDef
    }
}

impl NodeInfo for ImplTraitNode {
    fn node_type(&self) -> NodeType {
        NodeType::ImplTraitDef
    }
}
