use crate::{DefId, IdentId, NodeInfo, NodeType, Span, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplTraitNode {
    pub generic_parameters: Vec<IdentId>,
    pub trait_ty: UncheckedType,
    pub ty: UncheckedType,
    pub body: Vec<DefId>,
    pub span: Span,
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
