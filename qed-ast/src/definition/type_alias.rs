use crate::{IdentId, NodeInfo, NodeType, UncheckedType, Visibility};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub visibility: Visibility,
}

impl NodeInfo for TypeAliasNode {
    fn node_type(&self) -> NodeType {
        NodeType::TypeAliasDef
    }
}
