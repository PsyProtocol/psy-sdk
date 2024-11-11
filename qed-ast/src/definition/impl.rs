use crate::{AstVisitor, FunctionNode, IdentId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub ty: IdentId,
    pub body: Vec<FunctionNode>,
}

impl ImplNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
    ) -> V::StmtResult {
        visitor.visit_impl(self)
    }
}
