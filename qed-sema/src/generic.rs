use crate::{IdentId, Span, TypeId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedGenericParameter {
    pub name: IdentId,
    pub constraints: Vec<TypeId>,
    pub span: Span,
}

impl CheckedGenericParameter {
    pub fn new(name: IdentId, constraints: Vec<TypeId>, span: Span) -> Self {
        Self {
            name,
            constraints,
            span,
        }
    }
}
