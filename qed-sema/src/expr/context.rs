use qed_ast::{ExprId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub enum CheckedContextNode {
    GetUserId {
        type_id: TypeId,
    },
    GetContractId {
        type_id: TypeId,
    },
    GetCheckpointId {
        type_id: TypeId,
    },
    GetLastNonce {
        type_id: TypeId,
    },
    GetUserPublicKeyHash {
        type_id: TypeId,
    },
    GetStateHashAt {
        slot_index: ExprId,
        type_id: TypeId,
    },
    GetOtherContractStateHashAt {
        contract_state_tree_height: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        type_id: TypeId,
    },
    GetOtherUserContractStateHashAt {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        type_id: TypeId,
    },
    CSetStateHashAt {
        slot_index: ExprId,
        new_value: ExprId,
        type_id: TypeId,
    },
}

impl NodeInfo for CheckedContextNode {
    fn node_type(&self) -> NodeType {
        NodeType::ContextExpr
    }
}
