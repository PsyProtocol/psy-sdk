use crate::{IdentId, NodeInfo, NodeType, Span, UncheckedType, Visibility};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub visibility: Visibility,
    pub span: Span,
}

impl NodeInfo for TypeAliasNode {
    fn node_type(&self) -> NodeType {
        NodeType::TypeAliasDef
    }
}
