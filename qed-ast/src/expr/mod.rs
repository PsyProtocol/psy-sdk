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

use crate::{AstVisitor, NodeType, ValueNode};
use enum_as_inner::EnumAsInner;
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
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
    pub fn node_type(&self) -> NodeType {
        match self {
            ExprNode::Path(_) => NodeType::PathExpr,
            ExprNode::Value(_) => NodeType::ValueExpr,
            ExprNode::Binary(_) => NodeType::BinaryExpr,
            ExprNode::Unary(_) => NodeType::UnaryExpr,
            ExprNode::Call(_) => NodeType::CallExpr,
            ExprNode::Cast(_) => NodeType::CastExpr,
            ExprNode::IndexAccess(_) => NodeType::IndexAccessExpr,
            ExprNode::MemberAccess(_) => NodeType::MemberAccessExpr,
        }
    }
}
