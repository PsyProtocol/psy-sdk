use crate::{DefId, GenericParameter, IdentId, NodeInfo, NodeType, Span, Visibility};

#[derive(Clone, Debug, PartialEq)]
pub struct TraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<GenericParameter>,
    pub body: Vec<DefId>,
    pub visibility: Visibility,
    pub span: Span,
}

impl NodeInfo for TraitNode {
    fn node_type(&self) -> NodeType {
        NodeType::TraitDef
    }
}
