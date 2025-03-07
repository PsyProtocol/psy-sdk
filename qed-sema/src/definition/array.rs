use crate::{ScopeId, TypeId};
use qed_ast::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedArrayNode {
    pub inner_ty: TypeId,
    pub size_ty: TypeId,
    pub scope_id: ScopeId,
    pub implementations: Vec<TypeId>,
}
