use std::collections::HashMap;

use qed_ast::IdentId;

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub fields: Vec<(IdentId, TypeId)>,
    pub scope_id: ScopeId,
    pub implementations: Vec<TypeId>,
}
