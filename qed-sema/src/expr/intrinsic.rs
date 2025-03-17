use qed_ast::{ExprId, NodeInfo, NodeType, Span};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub enum CheckedIntrinsicExprNode {
    GetUserId {
        type_id: TypeId,
        span: Span,
    },
    GetContractId {
        type_id: TypeId,
        span: Span,
    },
    GetCheckpointId {
        type_id: TypeId,
        span: Span,
    },
    GetLastNonce {
        type_id: TypeId,
        span: Span,
    },
    GetUserPublicKeyHash {
        type_id: TypeId,
        span: Span,
    },
    GetStateHashAt {
        slot_index: ExprId,
        type_id: TypeId,
        span: Span,
    },
    GetOtherContractStateHashAt {
        contract_state_tree_height: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        type_id: TypeId,
        span: Span,
    },
    GetOtherUserContractStateHashAt {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        type_id: TypeId,
        span: Span,
    },
    CSetStateHashAt {
        slot_index: ExprId,
        new_value: ExprId,
        type_id: TypeId,
        span: Span,
    },
    Read {
        offset: ExprId,
        type_id: TypeId,
        span: Span,
    },
    Write {
        offset: ExprId,
        value: ExprId,
        type_id: TypeId,
        span: Span,
    },
    Hash {
        data: ExprId,
        type_id: TypeId,
        span: Span,
    },
}

impl NodeInfo for CheckedIntrinsicExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicExpr
    }
}
