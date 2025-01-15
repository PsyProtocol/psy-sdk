use crate::{AstVisitor, BlockNode, IdentId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, bool, UncheckedType)>,
    pub generic_parameters: Vec<IdentId>,
    pub body: Option<BlockNode>,
    pub return_type: Option<UncheckedType>,
}

impl FunctionNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
        ctx: &mut V::Context,
    ) -> Result<V::StmtResult, V::Error> {
        visitor.visit_function(self, ctx)
    }
}
