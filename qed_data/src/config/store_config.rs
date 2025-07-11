use kvq::adapters::standard::KVQStandardAdapter;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::poseidon::PoseidonHash};
use qed_core::{
    config::network_constants::{
        CHECKPOINT_TREE_HEIGHT, CONTRACT_FUNCTION_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT,
        GLOBAL_DEPOSIT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, GLOBAL_WITHDRAWAL_TREE_HEIGHT,
    },
    data::qhashout::QHashOut,
};
use qed_crypto::hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore};
use crate::{qdata::{
    checkpoint::{QEDCheckpointLeaf, QEDL2BlockState}, checkpoint_id_key::CheckpointTableIdKey, contract::{ContractCodeDefinition, QEDContractLeaf}, hash_cache_result::QEDHashHelperResult, hash_key::Hash4x64Key, hash_key_with_id::Hash4x64KeyWithId, u64_key::U64TableKey, user::QEDUserLeaf, user_public_key::QEDUserPublicKeyRecord
}, qsync::coordinator::QEDCheckpointSyncInfoCompact};

use crate::models::{
    checkpoint::{block_state::L2BlockStatesModel, checkpoint_hash::QEDCheckpointHashHelperModel, checkpoint_leaf::QEDCheckpointLeafModel, sync_info::QEDCheckpointSyncInfoModel, user_public_keys::QEDUserPublicKeyHelperModel},
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
pub const USER_REGISTRATION_TREE_ID: u8 = 9u8;

// Protocol tree table types - separated for better data management
pub const CHECKPOINT_TREE_TABLE_TYPE: u16 = 1;
pub const USER_TREE_TABLE_TYPE: u16 = 2;
pub const CONTRACT_TREE_TABLE_TYPE: u16 = 3;
pub const CONTRACT_FUNCTION_TREE_TABLE_TYPE: u16 = 4;
pub const DEPOSIT_TREE_TABLE_TYPE: u16 = 5;
pub const WITHDRAWAL_TREE_TABLE_TYPE: u16 = 6;
pub const USER_REGISTRATION_TREE_TABLE_TYPE: u16 = 7;

// User contract tree table types
pub const USER_CONTRACT_TREE_TABLE_TYPE: u16 = 8;
pub const USER_CONTRACT_STATE_TREE_TABLE_TYPE: u16 = 9;

// Leaf table types
pub const USER_LEAF_TABLE_TYPE: u16 = 10;
pub const CHECKPOINT_LEAF_TABLE_TYPE: u16 = 11;
pub const CHECKPOINT_BLOCK_STATE_TABLE_TYPE: u16 = 12;
pub const CONTRACT_LEAF_TABLE_TYPE: u16 = 13;
pub const CONTRACT_CODE_TABLE_TYPE: u16 = 14;

// Helper table types
pub const CHECKPOINT_SYNC_INFO_TABLE_TYPE: u16 = 15;
pub const CHECKPOINT_HASH_HELPER_TABLE_TYPE: u16 = 16;
pub const USER_PUBLIC_KEY_HELPER_TABLE_TYPE: u16 = 17;

// Legacy - kept for backward compatibility, should not be used for new trees
pub const PROTOCOL_TREE_TABLE_TYPE: u16 = 100;

pub type QEDFelt = GoldilocksField;
pub type QEDHash = QHashOut<QEDFelt>;
pub type QEDHasher = PoseidonHash;
pub type QEDMerkleProof = MerkleProofCore<QEDHash>;
pub type QEDDeltaMerkleProof = DeltaMerkleProofCore<QEDHash>;
pub type QCheckpointLeaf = QEDCheckpointLeaf<QEDFelt>;
pub type QCheckpointSyncInfoCompact = QEDCheckpointSyncInfoCompact<QEDFelt>;
pub type QUserPublicKeyRecord = QEDUserPublicKeyRecord<QEDFelt>;
//pub type QEDParams = QEDTestnetTreeConfig;

pub type UserLeafTableStore<S, A = KVQStandardAdapter<S, CheckpointTableIdKey<USER_LEAF_TABLE_TYPE>, QEDUserLeaf<QEDFelt>>> = UserLeafModel<
    USER_LEAF_TABLE_TYPE,
    S,
    A,
>;
pub type ContractLeafTableStore<S, A = KVQStandardAdapter<S, CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>, QEDContractLeaf<QEDFelt>>> = ContractLeafModel<
    CONTRACT_LEAF_TABLE_TYPE,
    S,
    A,
>;
pub type ContractCodeTableStore<S, A = KVQStandardAdapter<S, CheckpointTableIdKey<CONTRACT_CODE_TABLE_TYPE>, ContractCodeDefinition>> = ContractCodeModel<
    CONTRACT_CODE_TABLE_TYPE,
    S,
    A,
>;

pub type CheckpointLeafTableStore<S, A = KVQStandardAdapter<S, U64TableKey<CHECKPOINT_LEAF_TABLE_TYPE>, QCheckpointLeaf>> = QEDCheckpointLeafModel<
    CHECKPOINT_LEAF_TABLE_TYPE,
    S,
    A,
>;

pub type L2BlockStateTableStore<S, A = KVQStandardAdapter<S, U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>, QEDL2BlockState>> = L2BlockStatesModel<
    CHECKPOINT_BLOCK_STATE_TABLE_TYPE,
    S,
    A,
>;


pub type CheckpointSyncInfoTableStore<S, A = KVQStandardAdapter<S, U64TableKey<CHECKPOINT_SYNC_INFO_TABLE_TYPE>, QCheckpointSyncInfoCompact>> = QEDCheckpointSyncInfoModel<
    CHECKPOINT_SYNC_INFO_TABLE_TYPE,
    S,
    A,
>;

pub type CheckpointHashHelperTableStore<S, A = KVQStandardAdapter<S, Hash4x64Key<CHECKPOINT_HASH_HELPER_TABLE_TYPE>, QEDHashHelperResult>> = QEDCheckpointHashHelperModel<
    CHECKPOINT_HASH_HELPER_TABLE_TYPE,
    S,
    A,
>;

pub type UserPublicKeyTableStore<S, A = KVQStandardAdapter<S, Hash4x64KeyWithId<USER_PUBLIC_KEY_HELPER_TABLE_TYPE>, QUserPublicKeyRecord>> = QEDUserPublicKeyHelperModel<
    USER_PUBLIC_KEY_HELPER_TABLE_TYPE,
    S,
    A,
>;


// Generic protocol tree template - no longer used directly
pub type ProtocolTreeStore<S, const TREE_ID: u8, const HEIGHT: u8, const TABLE_TYPE: u16, A = KVQStandardAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, QEDHash>> = KVQFixedConfigMerkleTreeModel<
    TREE_ID,
    HEIGHT,
    0,
    0,
    TABLE_TYPE,
    false,
    S,
    A,
    QEDHash,
    QEDHasher,
>;

pub type UserContractTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<USER_CONTRACT_TREE_TABLE_TYPE>, QEDHash>> = KVQSemiFixedConfigMerkleTreeModel<
    USER_CONTRACT_TREE_ID,
    GLOBAL_CONTRACT_TREE_HEIGHT,
    0,
    USER_CONTRACT_TREE_TABLE_TYPE,
    false,
    S,
    A,
    QEDHash,
    QEDHasher,
>;

pub type BaseContractStateTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<USER_CONTRACT_STATE_TREE_TABLE_TYPE>, QEDHash>> = KVQMerkleTreeModel<
    USER_CONTRACT_STATE_TREE_TABLE_TYPE,
    false,
    S,
    A,
    QEDHash,
    QEDHasher,
>;

// Protocol tree stores with their own table types
pub type UserRegistrationTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<USER_REGISTRATION_TREE_TABLE_TYPE>, QEDHash>> = ProtocolTreeStore<S, USER_REGISTRATION_TREE_ID, GLOBAL_USER_TREE_HEIGHT, USER_REGISTRATION_TREE_TABLE_TYPE, A>;
pub type CheckpointTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<CHECKPOINT_TREE_TABLE_TYPE>, QEDHash>> = ProtocolTreeStore<S, CHECKPOINT_TREE_ID, CHECKPOINT_TREE_HEIGHT, CHECKPOINT_TREE_TABLE_TYPE, A>;
pub type UserTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<USER_TREE_TABLE_TYPE>, QEDHash>> = ProtocolTreeStore<S, USER_TREE_ID, GLOBAL_USER_TREE_HEIGHT, USER_TREE_TABLE_TYPE, A>;
pub type ContractTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<CONTRACT_TREE_TABLE_TYPE>, QEDHash>> = ProtocolTreeStore<S, CONTRACT_TREE_ID, GLOBAL_CONTRACT_TREE_HEIGHT, CONTRACT_TREE_TABLE_TYPE, A>;

pub type ContractFunctionTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<CONTRACT_FUNCTION_TREE_TABLE_TYPE>, QEDHash>> = KVQSemiFixedConfigMerkleTreeModel<
    CONTRACT_FUNCTION_TREE_ID,
    CONTRACT_FUNCTION_TREE_HEIGHT,
    0,
    CONTRACT_FUNCTION_TREE_TABLE_TYPE,
    false,
    S,
    A,
    QEDHash,
    QEDHasher,
>;

pub type DepositTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<DEPOSIT_TREE_TABLE_TYPE>, QEDHash>> = ProtocolTreeStore<S, DEPOSIT_TREE_ID, GLOBAL_DEPOSIT_TREE_HEIGHT, DEPOSIT_TREE_TABLE_TYPE, A>;
pub type WithdrawalTreeStore<S, A = KVQStandardAdapter<S, KVQMerkleNodeKey<WITHDRAWAL_TREE_TABLE_TYPE>, QEDHash>> = ProtocolTreeStore<S, WITHDRAWAL_TREE_ID, GLOBAL_WITHDRAWAL_TREE_HEIGHT, WITHDRAWAL_TREE_TABLE_TYPE, A>;

// GLOBAL_CONTRACT_TREE_HEIGHT-th zero hash
#[cfg(test)]
mod tests {
    use qed_core::config::network_constants::DEFAULT_USER_STATE_TREE_ROOT;
    use qed_core::config::network_constants::GLOBAL_CONTRACT_TREE_HEIGHT;
    use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
    use qed_crypto::hash::traits::hasher::PoseidonHasher;


    #[test]
    fn check_default_user_state_tree_root() {

        let expected_empty_user_state_tree_root = PoseidonHasher::get_zero_hash(GLOBAL_CONTRACT_TREE_HEIGHT as usize);
        assert_eq!(DEFAULT_USER_STATE_TREE_ROOT, expected_empty_user_state_tree_root, "DEFAULT_USER_STATE_TREE_ROOT does not match the expected value");
        // TODO: make sure the default user tree root is correct
    }
}
