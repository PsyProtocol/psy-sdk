use indexmap::IndexMap;

use crate::{AttrNode, IdentId, NodeInfo, NodeType, Span, UncheckedType, Visibility};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub fields: IndexMap<IdentId, (UncheckedType, Visibility)>,
    pub attrs: Vec<AttrNode>,
    pub visibility: Visibility,
    pub span: Span,
}

impl NodeInfo for StructNode {
    fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
