use std::fmt::Display;

use crate::{AstVisitor, ExprId};

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Neg,
    Not,
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOperator::Neg => write!(f, "-"),
            UnaryOperator::Not => write!(f, "!"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryNode {
    pub operator: UnaryOperator,
    pub rhs: ExprId,
}

impl UnaryNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &mut self,
        visitor: &mut V,
        ctx: &mut V::Context,
    ) -> Result<V::ExprResult, V::Error> {
        visitor.visit_unary(self, ctx)
    }
}
