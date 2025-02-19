use crate::{IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasNode {
    pub name: IdentId,
    pub ty: UncheckedType,
}

impl NodeInfo for TypeAliasNode {
    fn node_type(&self) -> NodeType {
        NodeType::TypeAliasDef
    }
}
