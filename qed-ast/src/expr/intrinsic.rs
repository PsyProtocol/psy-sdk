use crate::{ExprId, Location, NodeInfo, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub enum IntrinsicExprNode {
    GetUserId {
        location: Location,
    },
    GetContractId {
        location: Location,
    },
    GetCheckpointId {
        location: Location,
    },
    GetLastNonce {
        location: Location,
    },
    GetUserPublicKeyHash {
        location: Location,
    },
    GetStateHashAt {
        slot_index: ExprId,
        location: Location,
    },
    GetOtherContractStateHashAt {
        contract_state_tree_height: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        location: Location,
    },
    GetOtherUserContractStateHashAt {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        location: Location,
    },
    CSetStateHashAt {
        slot_index: ExprId,
        new_value: ExprId,
        location: Location,
    },
    Read {
        offset: ExprId,
        location: Location,
    },
    Write {
        offset: ExprId,
        value: ExprId,
        location: Location,
    },
    Hash {
        data: ExprId,
        location: Location,
    },
}

impl NodeInfo for IntrinsicExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicExpr
    }
}
