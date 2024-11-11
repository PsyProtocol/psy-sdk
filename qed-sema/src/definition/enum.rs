use qed_ast::IdentId;

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub enum CheckedEnumVariant {
    Basic(IdentId),
    Tuple(IdentId, Vec<TypeId>),
    Struct(IdentId, Vec<(IdentId, TypeId)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedEnumNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub variants: Vec<CheckedEnumVariant>,
    pub scope_id: ScopeId,
}
