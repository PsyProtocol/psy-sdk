use qed_ast::IdentId;

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedVariable<T> {
    pub ty: TypeId,
    pub mutable: bool,
    pub cnst: bool,
    pub scope_id: ScopeId,
    pub value: Option<T>,
}

impl<T> CheckedVariable<T> {
    pub fn new(
        ty: TypeId,
        mutable: bool,
        cnst: bool,
        scope_id: ScopeId,
        value: Option<T>,
    ) -> CheckedVariable<T> {
        Self {
            ty,
            mutable,
            cnst,
            scope_id,
            value,
        }
    }
}
