use crate::{AstVisitor, DefId, FunctionNode, IdentId, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub trait_name: Option<IdentId>,
    pub ty: IdentId,
    pub body: Vec<DefId>,
}

impl ImplNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::ImplDef
    }
}
