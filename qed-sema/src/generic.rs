use crate::{IdentId, ScopeId, Span, TypeId, UncheckedType};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CheckedGenericParameter {
    pub name: IdentId,
    pub constraints: Vec<TypeId>,
    pub scope_id: ScopeId,
    pub span: Span,
}

impl CheckedGenericParameter {
    pub fn new(name: IdentId, constraints: Vec<TypeId>, scope_id: ScopeId, span: Span) -> Self {
        Self {
            name,
            constraints,
            scope_id,
            span,
        }
    }
}
