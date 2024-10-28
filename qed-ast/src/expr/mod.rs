mod binary;
mod call;
mod unary;
mod variable;

pub use binary::*;
pub use call::*;
pub use unary::*;
pub use variable::*;

use crate::{visitor::AstVisitor, ValueNode};
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum ExprNode<F> {
    Variable(VariableNode),
    Value(ValueNode<F>),
    Binary(BinaryNode),
    Unary(UnaryNode),
    Call(CallNode),
}

impl<F> ExprNode<F> {
    pub fn accept_visitor<V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::ExprResult {
        visitor.visit_expr(self)
    }
}
