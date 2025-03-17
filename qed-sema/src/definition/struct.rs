use indexmap::IndexMap;
use qed_ast::{IdentId, NodeInfo, NodeType, Span, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub fields: IndexMap<IdentId, CheckedStructField>,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructField {
    pub ty: TypeId,
    pub visibility: Visibility,
    pub span: Span,
}

impl NodeInfo for CheckedStructNode {
    fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
