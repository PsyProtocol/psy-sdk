use qed_ast::{IdentId, Location, TypeQualifier};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug)]
pub struct CheckedVariable<F: Clone + From<u32> + ContextFelt> {
    pub name: IdentId,
    pub ty: TypeId,
    pub qualifier: TypeQualifier,
    pub scope_id: ScopeId,
    pub location: Location,
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Clone + From<u32> + ContextFelt> CheckedVariable<F> {
    pub fn new(
        name: IdentId,
        ty: TypeId,
        qualifier: TypeQualifier,
        scope_id: ScopeId,
        location: Location,
    ) -> CheckedVariable<F> {
        Self {
            name,
            ty,
            qualifier,
            scope_id,
            location,
            _phantom: std::marker::PhantomData,
        }
    }
}
