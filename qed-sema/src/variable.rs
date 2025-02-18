use crate::{CheckedValueRef, ScopeId, TypeId};

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
use std::fmt;

impl<T> fmt::Display for CheckedVariable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CheckedVariable {{ ty: {:?}, mutable: {}, cnst: {}, scope_id: {:?}, value: {} }}",
            self.ty,
            self.mutable,
            self.cnst,
            self.scope_id,
            match &self.value {
                Some(value) => "Some".to_string(),
                None => "None".to_string(),
            }
        )
    }
}
