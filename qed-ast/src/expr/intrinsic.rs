use crate::{ExprId, NodeInfo, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub enum IntrinsicExprNode {
    GetUserId,
    GetContractId,
    GetCheckpointId,
    GetLastNonce,
    GetUserPublicKeyHash,
    GetStateHashAt {
        slot_index: ExprId,
    },
    GetOtherContractStateHashAt {
        contract_state_tree_height: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
    },
    GetOtherUserContractStateHashAt {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
    },
    CSetStateHashAt {
        slot_index: ExprId,
        new_value: ExprId,
    },
    Read {
        offset: ExprId,
    },
    Write {
        offset: ExprId,
        value: ExprId,
    },
}

impl NodeInfo for IntrinsicExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicExpr
    }
}
