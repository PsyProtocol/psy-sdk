mod binary;
mod call;
mod cast;
mod index;
mod path;
mod unary;

pub use binary::*;
pub use call::*;
pub use cast::*;
pub use index::*;
pub use path::*;
pub use unary::*;

use crate::{AstVisitor, ValueNode};
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum ExprNode<F: Clone> {
    Path(PathNode),
    Value(ValueNode<F>),
    Binary(BinaryNode),
    Unary(UnaryNode),
    Call(CallNode),
    Cast(CastNode),
    IndexAccess(IndexAccessNode),
    MemberAccess(MemberAccessNode),
}

impl<F: Clone> ExprNode<F> {
    pub fn accept_visitor<C, V: AstVisitor<F, C>>(&self, visitor: &mut V) -> V::ExprResult {
        visitor.visit_expr(self)
    }
}
