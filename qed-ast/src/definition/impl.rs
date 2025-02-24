use crate::{DefId, IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub trait_ty: Option<UncheckedType>,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
}

impl NodeInfo for ImplNode {
    fn node_type(&self) -> NodeType {
        NodeType::ImplDef
    }
}
