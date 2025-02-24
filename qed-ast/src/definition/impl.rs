use crate::{DefId, IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplTraitNode {
    pub generic_parameters: Vec<IdentId>,
    pub trait_ty: UncheckedType,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
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
