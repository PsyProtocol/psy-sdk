use crate::{AstVisitor, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub stmts: Vec<StmtId>,
}

// impl BlockNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//         ctx: &mut V::Context,
//     ) -> Result<V::StmtResult, V::Error> {
//         visitor.visit_block(self, ctx)
//     }
// }
