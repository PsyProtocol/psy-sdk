use crate::{visitor::AstVisitor, BlockNode, ExprId};

#[derive(Debug, Clone, PartialEq)]
pub struct WhileNode {
    pub predicate: ExprId,
    pub body: BlockNode,
}

impl WhileNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_while(self)
    }
}
