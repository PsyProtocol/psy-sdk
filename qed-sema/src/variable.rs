use qed_ast::TypeQualifier;
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{CheckedValueRef, ScopeId, TypeId};

#[derive(Clone, Debug)]
pub struct CheckedVariable<F: Clone + From<u32> + ContextFelt> {
    pub ty: TypeId,
    pub qualifier: TypeQualifier,
    pub scope_id: ScopeId,
    pub value: Option<CheckedValueRef<F>>,
}

impl<F: Clone + From<u32> + ContextFelt> CheckedVariable<F> {
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
