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
use enum_as_inner::EnumAsInner;
pub use index::*;
pub use path::*;
pub use storage::*;
pub use unary::*;

use qed_ast::{ExprNode, IdentId, NodeInfo, NodeType};

use crate::{CheckedValueNode, ScopeId, TypeId, TypeKey, BOOL_TYPE, FELT_TYPE};
use strum::EnumTryAs;

#[derive(Debug, Clone, PartialEq, EnumAsInner, EnumTryAs)]
pub enum CheckedExprNode<F> {
    Path(CheckedPathNode),
    Value(CheckedValueNode<F>),
    Binary(CheckedBinaryNode),
    Unary(CheckedUnaryNode),
    Cast(CheckedCastNode),
    Call(CheckedCallNode),
    IndexAccess(CheckedIndexAccessNode),
    MemberAccess(CheckedMemberAccessNode),
    Storage(CheckedStorageReadNode),
}

impl<F> NodeInfo for CheckedExprNode<F> {
    fn node_type(&self) -> NodeType {
        match self {
            CheckedExprNode::Path(node) => node.node_type(),
            CheckedExprNode::Value(node) => node.node_type(),
            CheckedExprNode::Binary(node) => node.node_type(),
            CheckedExprNode::Unary(node) => node.node_type(),
            CheckedExprNode::Cast(node) => node.node_type(),
            CheckedExprNode::Call(node) => node.node_type(),
            CheckedExprNode::IndexAccess(node) => node.node_type(),
            CheckedExprNode::MemberAccess(node) => node.node_type(),
            CheckedExprNode::Storage(node) => node.node_type(),
        }
    }
}

impl<F> CheckedExprNode<F> {
    pub fn ty(&self) -> TypeId {
        match self {
            CheckedExprNode::Path(p) => p.type_id,
            CheckedExprNode::Value(v) => match v {
                CheckedValueNode::Felt(_) => FELT_TYPE,
                CheckedValueNode::Bool(_) => BOOL_TYPE,
                CheckedValueNode::Array(type_id, _) => type_id.clone(),
                CheckedValueNode::Struct(type_id, _) => type_id.clone(),
                CheckedValueNode::Type(type_id) => type_id.clone(),
            },
            CheckedExprNode::Binary(b) => b.type_id,
            CheckedExprNode::Unary(u) => u.type_id,
            CheckedExprNode::Cast(c) => c.target_type,
            CheckedExprNode::Call(c) => c.type_id,
            CheckedExprNode::IndexAccess(i) => i.type_id,
            CheckedExprNode::MemberAccess(m) => m.type_id,
            CheckedExprNode::Storage(s) => s.type_id,
        }
    }

    pub fn scope_id(&self) -> Option<ScopeId> {
        match self {
            CheckedExprNode::Path(p) => Some(p.scope_id),
            _ => None,
        }
    }

    pub fn name(&self) -> IdentId {
        match self {
            CheckedExprNode::Path(p) => p.name,
            _ => panic!("Expected path node"),
        }
    }
}
