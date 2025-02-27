use qed_ast::Visibility;

use crate::{IdentId, NodeInfo, NodeType, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedTypeAliasNode {
    pub name: IdentId,
    pub ty: TypeId,
    pub visibility: Visibility,
}

impl NodeInfo for CheckedTypeAliasNode {
    fn node_type(&self) -> NodeType {
        NodeType::TypeAliasDef
    }
}
