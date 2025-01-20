use crate::{AstVisitor, BlockNode, ExprId, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct WhileNode {
    pub predicate: ExprId,
    pub body: StmtId,
}

// impl WhileNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//         ctx: &mut V::Context,
//     ) -> Result<V::StmtResult, V::Error> {
//         visitor.visit_while(self, ctx)
//     }
// }
