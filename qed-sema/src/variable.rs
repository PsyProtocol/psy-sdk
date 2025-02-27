use qed_ast::TypeQualifier;

use crate::{CheckedValueRef, ScopeId, TypeId};

#[derive(Clone, Debug)]
pub struct CheckedVariable<F> {
    pub ty: TypeId,
    pub qualifier: TypeQualifier,
    pub scope_id: ScopeId,
    pub value: Option<CheckedValueRef<F>>,
}

impl<F> CheckedVariable<F> {
    pub fn new(
        ty: TypeId,
        qualifier: TypeQualifier,
        scope_id: ScopeId,
        value: Option<CheckedValueRef<F>>,
    ) -> CheckedVariable<F> {
        Self {
            ty,
            qualifier,
            scope_id,
            value,
        }
    }
}
