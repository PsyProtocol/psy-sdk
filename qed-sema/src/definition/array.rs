use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedArrayNode {
    pub inner_ty: TypeId,
    pub size: usize,
    pub scope_id: ScopeId,
}
