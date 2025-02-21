mod binary;
pub mod block;
mod call;
mod cast;
pub mod if_expr;
mod index;
mod intrinsic;
mod path;
mod unary;

pub use binary::*;
pub use block::*;
pub use call::*;
pub use cast::*;
use enum_as_inner::EnumAsInner;
pub use index::*;
pub use intrinsic::*;
pub use path::*;
pub use unary::*;

use qed_ast::{IdentId, NodeInfo, NodeType};

use crate::expr::block::CheckedBlockExprNode;
use crate::expr::if_expr::CheckedIfExprNode;
use crate::{CheckedValueNode, ScopeId, TypeId, BOOL_TYPE, FELT_TYPE};
use strum::EnumTryAs;

#[derive(Debug, Clone, PartialEq, EnumAsInner, EnumTryAs)]
pub enum CheckedExprNode<F> {
    Path(CheckedPathNode),
    Value(CheckedValueNode<F>),
    Binary(CheckedBinaryNode),
    Unary(CheckedUnaryNode),
    Cast(CheckedCastNode),
    Call(CheckedCallNode),
    MemberCall(CheckedMemberCallNode),
    IndexAccess(CheckedIndexAccessNode),
    MemberAccess(CheckedMemberAccessNode),
    Intrinsic(CheckedIntrinsicExprNode),
    BlockExpr(CheckedBlockExprNode),
    IfExpr(CheckedIfExprNode),
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
            CheckedExprNode::MemberCall(node) => node.node_type(),
            CheckedExprNode::IndexAccess(node) => node.node_type(),
            CheckedExprNode::MemberAccess(node) => node.node_type(),
            CheckedExprNode::Intrinsic(node) => node.node_type(),
            CheckedExprNode::BlockExpr(node) => node.node_type(),
            CheckedExprNode::IfExpr(node) => node.node_type(),
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
            CheckedExprNode::MemberCall(c) => c.type_id,
            CheckedExprNode::IndexAccess(i) => i.type_id,
            CheckedExprNode::MemberAccess(m) => m.type_id,
            CheckedExprNode::Intrinsic(i) => match i {
                CheckedIntrinsicExprNode::GetUserId { type_id } => type_id.clone(),
                CheckedIntrinsicExprNode::GetContractId { type_id } => type_id.clone(),
                CheckedIntrinsicExprNode::GetCheckpointId { type_id } => type_id.clone(),
                CheckedIntrinsicExprNode::GetLastNonce { type_id } => type_id.clone(),
                CheckedIntrinsicExprNode::GetUserPublicKeyHash { type_id } => type_id.clone(),
                CheckedIntrinsicExprNode::GetStateHashAt { type_id, .. } => type_id.clone(),
                CheckedIntrinsicExprNode::GetOtherContractStateHashAt { type_id, .. } => {
                    type_id.clone()
                }
                CheckedIntrinsicExprNode::GetOtherUserContractStateHashAt { type_id, .. } => {
                    type_id.clone()
                }
                CheckedIntrinsicExprNode::CSetStateHashAt { type_id, .. } => type_id.clone(),
                CheckedIntrinsicExprNode::Read { type_id, .. } => type_id.clone(),
                CheckedIntrinsicExprNode::Write {
                    offset,
                    value,
                    type_id,
                } => type_id.clone(),
                CheckedIntrinsicExprNode::Hash { type_id, .. } => type_id.clone(),
            },
            CheckedExprNode::IfExpr(i) => i.type_id,
            CheckedExprNode::BlockExpr(b) => b.type_id,
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
