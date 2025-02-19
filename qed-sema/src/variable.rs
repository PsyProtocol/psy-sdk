use crate::{CheckedValueRef, ScopeId, TypeId};

#[derive(Clone, Debug)]
pub struct CheckedVariable<F> {
    pub ty: TypeId,
    pub mutable: bool,
    pub scope_id: ScopeId,
    pub value: Option<CheckedValueRef<F>>,
}

impl<F> CheckedVariable<F> {
    pub fn new(
        ty: TypeId,
        mutable: bool,
        scope_id: ScopeId,
        value: Option<CheckedValueRef<F>>,
    ) -> CheckedVariable<F> {
        Self {
            ty,
            mutable,
            scope_id,
            value,
        }
    }
}
