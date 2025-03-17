use crate::{ExprId, NodeInfo, NodeType, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum IntrinsicExprNode {
    GetUserId {
        span: Span,
    },
    GetContractId {
        span: Span,
    },
    GetCheckpointId {
        span: Span,
    },
    GetLastNonce {
        span: Span,
    },
    GetUserPublicKeyHash {
        span: Span,
    },
    GetStateHashAt {
        slot_index: ExprId,
        span: Span,
    },
    GetOtherContractStateHashAt {
        contract_state_tree_height: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        span: Span,
    },
    GetOtherUserContractStateHashAt {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        span: Span,
    },
    CSetStateHashAt {
        slot_index: ExprId,
        new_value: ExprId,
        span: Span,
    },
    Read {
        offset: ExprId,
        span: Span,
    },
    Write {
        offset: ExprId,
        value: ExprId,
        span: Span,
    },
    Hash {
        data: ExprId,
        span: Span,
    },
}

impl NodeInfo for IntrinsicExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicExpr
    }
}
