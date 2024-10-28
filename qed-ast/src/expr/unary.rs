use crate::{arena::ExprId, visitor::AstVisitor};

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Neg,
    Not,
    // BitNot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryNode {
    pub operator: UnaryOperator,
    pub rhs: ExprId,
}

impl UnaryNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::ExprResult {
        visitor.visit_unary(self)
    }
}
