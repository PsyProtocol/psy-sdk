use indexmap::IndexMap;

use crate::{
    AttrNode, GenericParameter, IdentId, Location, NodeInfo, NodeType, UncheckedType, Visibility,
};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<GenericParameter>,
    pub fields: IndexMap<IdentId, StructField>,
    pub attrs: Vec<AttrNode>,
    pub visibility: Visibility,
    pub location: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructField {
    pub ty: UncheckedType,
    pub visibility: Visibility,
    pub location: Location,
}

impl NodeInfo for StructNode {
    fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
