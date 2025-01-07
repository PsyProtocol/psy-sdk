mod binary;
mod call;
mod index;
mod path;
mod unary;

pub use binary::*;
pub use call::*;
pub use index::*;
pub use path::*;
pub use unary::*;

use qed_ast::{ExprNode, IdentId};

use crate::{CheckedValueNode, TypeId, TypeKey, BOOL_TYPE, FELT_TYPE};
use strum::{EnumIs, EnumTryAs};

#[derive(Debug, Clone, PartialEq, EnumIs, EnumTryAs)]
pub enum CheckedExprNode<F> {
    Path(CheckedPathNode),
    Value(CheckedValueNode<F>),
    Binary(CheckedBinaryNode),
    Unary(CheckedUnaryNode),
    Call(CheckedCallNode),
    IndexAccess(CheckedIndexAccessNode),
    MemberAccess(CheckedMemberAccessNode),
}

impl<F> CheckedExprNode<F> {
    pub fn ty(&self) -> TypeId {
        match self {
            CheckedExprNode::Path(p) => p.type_id,
            CheckedExprNode::Value(v) => match v {
                CheckedValueNode::Felt(_) => FELT_TYPE,
                CheckedValueNode::Bool(_) => BOOL_TYPE,
                CheckedValueNode::Array(type_id, _, _) => type_id.clone(),
                CheckedValueNode::Struct(type_id, _) => type_id.clone(),
                CheckedValueNode::Type(type_id) => type_id.clone(),
            },
            CheckedExprNode::Binary(b) => b.type_id,
            CheckedExprNode::Unary(u) => u.type_id,
            CheckedExprNode::Call(c) => c.type_id,
            CheckedExprNode::IndexAccess(i) => i.type_id,
            CheckedExprNode::MemberAccess(m) => m.type_id,
        }
    }
}
