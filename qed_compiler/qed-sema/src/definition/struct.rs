use indexmap::IndexMap;
use qed_ast::{Comment, Identifier, Location, NodeInfo, NodeType, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructNode {
    pub name: Identifier,
    pub generic_parameters: Vec<TypeId>,
    pub fields: IndexMap<Identifier, CheckedStructField>,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
    pub comments: Vec<Comment>,
    pub location: Location,
    pub type_id: TypeId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructField {
    pub ty: TypeId,
    pub visibility: Visibility,
    pub comments: Vec<Comment>,
    pub location: Location,
}

impl CheckedStructField {
    pub fn new(
        ty: TypeId,
        visibility: Visibility,
        comments: Vec<Comment>,
        location: Location,
    ) -> Self {
        Self {
            ty,
            visibility,
            comments,
            location,
        }
    }
}

impl NodeInfo for CheckedStructNode {
    fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
