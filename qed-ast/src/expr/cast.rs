use crate::{AstVisitor, ExprId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CastNode {
    pub value: ExprId,
    pub target_type: UncheckedType,
}

impl CastNode {
    pub fn new(value: ExprId, target_type: UncheckedType) -> Self {
        Self { value, target_type }
    }

    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
    ) -> V::ExprResult {
        visitor.visit_cast(self)
    }
}
