use crate::{arena::StmtId, visitor::AstVisitor};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub stmts: Vec<StmtId>,
}

impl BlockNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_block(self)
    }
}
