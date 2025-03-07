use indexmap::IndexMap;
use qed_ast::{IdentId, NodeInfo, NodeType, Span, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub enum CheckedEnumVariant {
    Basic(IdentId, TypeId),
    Tuple(IdentId, Vec<TypeId>),
    Struct(IdentId, IndexMap<IdentId, (TypeId, Visibility)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedEnumNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub variants: Vec<CheckedEnumVariant>,
    pub scope_id: ScopeId,
    pub implementations: Vec<TypeId>,
    pub visibility: Visibility,
    pub span: Span,
}

impl NodeInfo for CheckedEnumNode {
    fn node_type(&self) -> NodeType {
        NodeType::EnumDef
    }
}
