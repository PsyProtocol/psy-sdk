use crate::{IdentId, Location, NodeInfo, NodeType, UncheckedType, Visibility};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub visibility: Visibility,
    pub location: Location,
}

impl NodeInfo for TypeAliasNode {
    fn node_type(&self) -> NodeType {
        NodeType::TypeAliasDef
    }
}
