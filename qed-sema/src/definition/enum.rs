use indexmap::IndexMap;
use qed_ast::{IdentId, Location, NodeInfo, NodeType, Visibility};

use crate::{CheckedStructField, ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub enum CheckedEnumVariant {
    Basic(IdentId, TypeId),
    Tuple(IdentId, Vec<TypeId>),
    Struct(IdentId, IndexMap<IdentId, CheckedStructField>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedEnumNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub variants: Vec<CheckedEnumVariant>,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
    pub location: Location,
}

impl NodeInfo for CheckedEnumNode {
    fn node_type(&self) -> NodeType {
        NodeType::EnumDef
    }
}
