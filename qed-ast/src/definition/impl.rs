use crate::{DefId, IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub trait_name: Option<IdentId>,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
}

impl NodeInfo for ImplNode {
    fn node_type(&self) -> NodeType {
        NodeType::ImplDef
    }
}
