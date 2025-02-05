mod binary;
mod call;
mod cast;
mod index;
mod path;
mod storage;
mod unary;

pub use binary::*;
pub use call::*;
pub use cast::*;
pub use index::*;
pub use path::*;
pub use storage::*;
pub use unary::*;

use crate::{AstVisitor, DefId, ExprId, NodeInfo, NodeType, ValueNode};
use enum_as_inner::EnumAsInner;

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum ExprNode<F: Clone + From<u32>> {
    Path(PathNode),
    Value(ValueNode<F>),
    Binary(BinaryNode),
    Unary(UnaryNode),
    Call(CallNode),
    Cast(CastNode),
    IndexAccess(IndexAccessNode),
    MemberAccess(MemberAccessNode),
    Storage(StorageReadNode),
}

impl<F: Clone + From<u32>> NodeInfo for ExprNode<F> {
    fn node_type(&self) -> NodeType {
        match self {
            Self::Path(node) => node.node_type(),
            Self::Value(node) => node.node_type(),
            Self::Binary(node) => node.node_type(),
            Self::Unary(node) => node.node_type(),
            Self::Call(node) => node.node_type(),
            Self::Cast(node) => node.node_type(),
            Self::IndexAccess(node) => node.node_type(),
            Self::MemberAccess(node) => node.node_type(),
            Self::Storage(node) => node.node_type(),
        }
    }

    fn as_expression(&self) -> Option<ExprId> {
        unreachable!()
    }

    fn as_definition(&self) -> Option<DefId> {
        unreachable!()
    }
}
