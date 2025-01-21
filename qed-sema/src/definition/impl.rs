use qed_ast::IdentId;

use crate::{CheckedFunctionNode, ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedImplNode {
    pub generic_parameters: Vec<TypeId>,
    pub trait_name: Option<IdentId>,
    pub ty: IdentId,
    pub body: Vec<CheckedFunctionNode>,
    pub scope_id: ScopeId,
}
