use crate::{IdentId, Span, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParameter {
    pub name: IdentId,
    pub constraints: Vec<UncheckedType>,
    pub span: Span,
}

impl GenericParameter {
    pub fn new(name: IdentId, constraints: Vec<UncheckedType>, span: Span) -> Self {
        Self {
            name,
            constraints,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericQualifier {
    pub is_const: bool,
}

impl GenericQualifier {
    pub fn new(is_const: bool) -> Self {
        Self { is_const }
    }
}
