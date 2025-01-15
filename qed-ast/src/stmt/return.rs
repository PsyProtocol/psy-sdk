use crate::{AstVisitor, ExprId};

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnNode(pub Option<ExprId>);

impl ReturnNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
        ctx: &mut V::Context,
    ) -> Result<V::StmtResult, V::Error> {
        visitor.visit_return(self, ctx)
    }
}
