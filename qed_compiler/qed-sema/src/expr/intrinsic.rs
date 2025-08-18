use qed_ast::{ExprId, Location, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub enum CheckedIntrinsicExprNode {
    GetUserId {
        type_id: TypeId,
        location: Location,
    },
    GetContractId {
        type_id: TypeId,
        location: Location,
    },
    GetCheckpointId {
        type_id: TypeId,
        location: Location,
    },
    GetCheckpointStats {
        checkpoint_id: ExprId,
        type_id: TypeId,
        location: Location,
    },
    GetRegisterUsersRoot {
        checkpoint_id: ExprId,
        type_id: TypeId,
        location: Location,
    },
    GetGutasRoot {
        checkpoint_id: ExprId,
        type_id: TypeId,
        location: Location,
    },
    GetDeployContractsRoot {
        checkpoint_id: ExprId,
        type_id: TypeId,
        location: Location,
    },
    GetLastNonce {
        type_id: TypeId,
        location: Location,
    },
    GetUserPublicKeyHash {
        type_id: TypeId,
        location: Location,
    },
    GetStateHashAt {
        slot_index: ExprId,
        type_id: TypeId,
        location: Location,
    },
    GetOtherContractStateHashAt {
        contract_state_tree_height: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        type_id: TypeId,
        location: Location,
    },
    GetOtherUserContractStateHashAt {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        slot_index: ExprId,
        type_id: TypeId,
        location: Location,
    },
    CSetStateHashAt {
        slot_index: ExprId,
        new_value: ExprId,
        type_id: TypeId,
        location: Location,
    },
    MemTransmute {
        data: ExprId,
        target_type: TypeId,
        location: Location,
    },
    MemSizeOf {
        query_type: TypeId,
        type_id: TypeId,
        location: Location,
    },
    StorageRead {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        offset: ExprId,
        type_id: TypeId,
        location: Location,
    },
    StorageReadRange {
        contract_state_tree_height: ExprId,
        user_id: ExprId,
        contract_id: ExprId,
        offset: ExprId,
        length: ExprId,
        type_id: TypeId,
        location: Location,
    },
    StorageWrite {
        offset: ExprId,
        value: ExprId,
        type_id: TypeId,
        location: Location,
    },
    StorageWriteRange {
        offset: ExprId,
        values: ExprId,
        type_id: TypeId,
        location: Location,
    },
    Hash {
        data: ExprId,
        type_id: TypeId,
        location: Location,
    },
    HashTwoToOne {
        left: ExprId,
        right: ExprId,
        type_id: TypeId,
        location: Location,
    },
    InvokeSync {
        contract_id: ExprId,
        method_id: ExprId,
        inputs: ExprId,
        type_id: TypeId,
        location: Location,
    },
    InvokeDeferred {
        contract_id: ExprId,
        method_id: ExprId,
        inputs: ExprId,
        type_id: TypeId,
        location: Location,
    },
    CheckSecpSign {
        pub_key: ExprId,
        msg: ExprId,
        sig: ExprId,
        type_id: TypeId,
        location: Location,
    },
}

impl NodeInfo for CheckedIntrinsicExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IntrinsicExpr
    }
}
