use std::collections::HashMap;

use qed_ast::{IdentId, NodeInfo, NodeType, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub fields: Vec<(IdentId, TypeId, Visibility)>,
    pub scope_id: ScopeId,
    pub implementations: Vec<TypeId>,
    pub visibility: Visibility,
}

impl NodeInfo for CheckedStructNode {
    fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
