use crate::{AstVisitor, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub stmts: Vec<StmtId>,
}

impl BlockNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
    ) -> V::StmtResult {
        visitor.visit_block(self)
    }
}
