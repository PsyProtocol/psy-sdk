use crate::{arena::ExprId, visitor::AstVisitor};

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnNode(pub Option<ExprId>);

impl ReturnNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_return(self)
    }
}
