use std::{cell::RefCell, rc::Rc};

use qed_ast::{ExprId, IdentId};

use crate::{CheckedExprNode, CheckedValue, CheckedValueNode, CheckedValueRef, ScopeId, TypeId};

#[derive(Clone, Debug)]
pub struct CheckedVariable<F> {
    pub ty: TypeId,
    pub mutable: bool,
    pub cnst: bool,
    pub scope_id: ScopeId,
    pub value: Option<CheckedValueRef<F>>,
}

impl<F> CheckedVariable<F> {
    pub fn new(
        ty: TypeId,
        mutable: bool,
        cnst: bool,
        scope_id: ScopeId,
        value: Option<CheckedValueRef<F>>,
    ) -> CheckedVariable<F> {
        Self {
            ty,
            mutable,
            cnst,
            scope_id,
            value,
        }
    }
}
