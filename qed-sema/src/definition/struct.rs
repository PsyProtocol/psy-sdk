use indexmap::IndexMap;
use qed_ast::{Identifier, Location, NodeInfo, NodeType, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructNode {
    pub name: Identifier,
    pub generic_parameters: Vec<TypeId>,
    pub fields: IndexMap<Identifier, CheckedStructField>,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
    pub location: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructField {
    pub ty: TypeId,
    pub visibility: Visibility,
    pub location: Location,
}

impl NodeInfo for CheckedStructNode {
    fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
