use crate::{AstVisitor, ExprId, IdentId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct VariableNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub mutable: bool,
    pub cnst: bool,
    pub value: ExprId,
}

impl VariableNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
    ) -> V::StmtResult {
        visitor.visit_variable(self)
    }
}
