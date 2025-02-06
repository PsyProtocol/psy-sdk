use kvq::adapters::standard::KVQStandardAdapter;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::{hash_types::HashOut, poseidon::PoseidonHash}};
use qed_core::{
    config::network_constants::{
        CHECKPOINT_TREE_HEIGHT, CONTRACT_FUNCTION_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT,
        GLOBAL_DEPOSIT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, GLOBAL_WITHDRAWAL_TREE_HEIGHT,
    },
    data::qhashout::QHashOut,
};
use qed_crypto::hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore};
use qed_data::qdata::{
    checkpoint::{QEDCheckpointLeaf, QEDL2BlockState},
    checkpoint_id_key::CheckpointTableIdKey,
    contract::{ContractCodeDefinition, QEDContractLeaf},
    u64_key::U64TableKey,
    user::QEDUserLeaf,
};

use crate::models::{
    checkpoint::{block_state::L2BlockStatesModel, checkpoint_leaf::QEDCheckpointLeafModel},
    contract::{contract_code::ContractCodeModel, contract_leaf::ContractLeafModel},
    kvq_merkle::{
        key::KVQMerkleNodeKey,
        model::{
            KVQFixedConfigMerkleTreeModel, KVQMerkleTreeModel, KVQSemiFixedConfigMerkleTreeModel,
        },
    },
    user::user_leaf::UserLeafModel,
};
pub const MAX_CHECKPOINT: u64 = 0xfffffffffffffff1u64;
pub const CHECKPOINT_TREE_ID: u8 = 1u8;
pub const USER_TREE_ID: u8 = 2u8;
pub const CONTRACT_TREE_ID: u8 = 3u8;
pub const CONTRACT_FUNCTION_TREE_ID: u8 = 4u8;
pub const DEPOSIT_TREE_ID: u8 = 5u8;
pub const WITHDRAWAL_TREE_ID: u8 = 6u8;
pub const USER_CONTRACT_TREE_ID: u8 = 7u8;
pub const CONTRACT_STATE_TREE_ID: u8 = 8u8;

pub const PROTOCOL_TREE_TABLE_TYPE: u16 = 1;
pub const USER_CONTRACT_TREE_TABLE_TYPE: u16 = 2;
pub const USER_CONTRACT_STATE_TREE_TABLE_TYPE: u16 = 3;

pub const USER_LEAF_TABLE_TYPE: u16 = 4;
pub const CHECKPOINT_LEAF_TABLE_TYPE: u16 = 5;
pub const CHECKPOINT_BLOCK_STATE_TABLE_TYPE: u16 = 6;
pub const CONTRACT_LEAF_TABLE_TYPE: u16 = 7;

pub const CONTRACT_CODE_TABLE_TYPE: u16 = 8;

pub type QEDFelt = GoldilocksField;
pub type QEDHash = QHashOut<QEDFelt>;
pub type QEDHasher = PoseidonHash;
pub type QEDMerkleProof = MerkleProofCore<QEDHash>;
pub type QEDDeltaMerkleProof = DeltaMerkleProofCore<QEDHash>;
pub type QCheckpointLeaf = QEDCheckpointLeaf<QEDFelt>;

//pub type QEDParams = QEDTestnetTreeConfig;

pub type UserLeafTableStore<S> = UserLeafModel<
    USER_LEAF_TABLE_TYPE,
    S,
    KVQStandardAdapter<S, CheckpointTableIdKey<USER_LEAF_TABLE_TYPE>, QEDUserLeaf<QEDFelt>>,
>;
pub type ContractLeafTableStore<S> = ContractLeafModel<
    CONTRACT_LEAF_TABLE_TYPE,
    S,
    KVQStandardAdapter<S, CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>, QEDContractLeaf<QEDFelt>>,
>;
pub type ContractCodeTableStore<S> = ContractCodeModel<
    CONTRACT_CODE_TABLE_TYPE,
    S,
    KVQStandardAdapter<S, CheckpointTableIdKey<CONTRACT_CODE_TABLE_TYPE>, ContractCodeDefinition>,
>;

pub type CheckpointLeafTableStore<S> = QEDCheckpointLeafModel<
    CHECKPOINT_LEAF_TABLE_TYPE,
    S,
    KVQStandardAdapter<S, U64TableKey<CHECKPOINT_LEAF_TABLE_TYPE>, QCheckpointLeaf>,
>;

pub type L2BlockStateTableStore<S> = L2BlockStatesModel<
    CHECKPOINT_BLOCK_STATE_TABLE_TYPE,
    S,
    KVQStandardAdapter<S, U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>, QEDL2BlockState>,
>;

pub type ProtocolTreeStore<S, const TREE_ID: u8, const HEIGHT: u8> = KVQFixedConfigMerkleTreeModel<
    TREE_ID,
    HEIGHT,
    0,
    0,
    PROTOCOL_TREE_TABLE_TYPE,
    false,
    S,
    KVQStandardAdapter<S, KVQMerkleNodeKey<PROTOCOL_TREE_TABLE_TYPE>, QEDHash>,
    QEDHash,
    QEDHasher,
>;

pub type UserContractTreeStore<S> = KVQSemiFixedConfigMerkleTreeModel<
    USER_CONTRACT_TREE_ID,
    GLOBAL_CONTRACT_TREE_HEIGHT,
    0,
    USER_CONTRACT_TREE_TABLE_TYPE,
    false,
    S,
    KVQStandardAdapter<S, KVQMerkleNodeKey<USER_CONTRACT_TREE_TABLE_TYPE>, QEDHash>,
    QEDHash,
    QEDHasher,
>;
pub type BaseContractStateTreeStore<S> = KVQMerkleTreeModel<
    USER_CONTRACT_STATE_TREE_TABLE_TYPE,
    false,
    S,
    KVQStandardAdapter<S, KVQMerkleNodeKey<USER_CONTRACT_STATE_TREE_TABLE_TYPE>, QEDHash>,
    QEDHash,
    QEDHasher,
>;

pub type CheckpointTreeStore<S> = ProtocolTreeStore<S, CHECKPOINT_TREE_ID, CHECKPOINT_TREE_HEIGHT>;
pub type UserTreeStore<S> = ProtocolTreeStore<S, USER_TREE_ID, GLOBAL_USER_TREE_HEIGHT>;
pub type ContractTreeStore<S> = ProtocolTreeStore<S, CONTRACT_TREE_ID, GLOBAL_CONTRACT_TREE_HEIGHT>;
//pub type ContractFunctionTreeStore<S> = ProtocolTreeStore<S, CONTRACT_FUNCTION_TREE_ID, CONTRACT_FUNCTION_TREE_HEIGHT>;

pub type ContractFunctionTreeStore<S> = KVQSemiFixedConfigMerkleTreeModel<
    CONTRACT_FUNCTION_TREE_ID,
    CONTRACT_FUNCTION_TREE_HEIGHT,
    0,
    PROTOCOL_TREE_TABLE_TYPE,
    false,
    S,
    KVQStandardAdapter<S, KVQMerkleNodeKey<PROTOCOL_TREE_TABLE_TYPE>, QEDHash>,
    QEDHash,
    QEDHasher,
>;
pub type DepositTreeStore<S> = ProtocolTreeStore<S, DEPOSIT_TREE_ID, GLOBAL_DEPOSIT_TREE_HEIGHT>;
pub type WithdrawalTreeStore<S> = ProtocolTreeStore<S, WITHDRAWAL_TREE_ID, GLOBAL_WITHDRAWAL_TREE_HEIGHT>;

// GLOBAL_CONTRACT_TREE_HEIGHT-th zero hash
pub const DEFAULT_USER_STATE_TREE_ROOT: QHashOut<GoldilocksField> = QHashOut::<QEDFelt>(
    HashOut {
        elements: [
            GoldilocksField(3896366420105793420),
            GoldilocksField(17410332186442776169),
            GoldilocksField(7329967984378645716),
            GoldilocksField(6310665049578686403),
        ],
    }
);


#[cfg(test)]
mod tests {
    use qed_core::config::network_constants::GLOBAL_CONTRACT_TREE_HEIGHT;
    use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
    use qed_crypto::hash::traits::hasher::PoseidonHasher;

    use crate::config::store_config::DEFAULT_USER_STATE_TREE_ROOT;

    #[test]
    fn check_default_user_state_tree_root() {
        
        let expected_empty_user_state_tree_root = PoseidonHasher::get_zero_hash(GLOBAL_CONTRACT_TREE_HEIGHT as usize);
        assert_eq!(DEFAULT_USER_STATE_TREE_ROOT, expected_empty_user_state_tree_root, "DEFAULT_USER_STATE_TREE_ROOT does not match the expected value");
        // TODO: make sure the default user tree root is correct
    }
}
