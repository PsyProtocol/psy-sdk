use kvq::adapters::standard::KVQStandardAdapter;
use qed_core::config::network_constants::DEFERRED_TRANSACTION_TREE_HEIGHT;

use qed_data::{config::store_config::{QEDHash, QEDHasher}, models::kvq_merkle::{key::KVQMerkleNodeKey, model::KVQFixedConfigMerkleTreeModel}};


const LOCAL_PROVING_SESSION_TREE_TABLE_TYPE: u16 = 0xFE01;

pub type LocalProvingSessionTreeStore<S, const TREE_ID: u8, const HEIGHT: u8> = KVQFixedConfigMerkleTreeModel<
    TREE_ID,
    HEIGHT,
    0,
    0,
    LOCAL_PROVING_SESSION_TREE_TABLE_TYPE,
    false,
    S,
    KVQStandardAdapter<S, KVQMerkleNodeKey<LOCAL_PROVING_SESSION_TREE_TABLE_TYPE>, QEDHash>,
    QEDHash,
    QEDHasher,
>;


pub const LPS_DEFERRED_TRANSACTION_TREE_ID: u8 = 1;


pub type LPSDeferredTransactionTreeStore<S> = LocalProvingSessionTreeStore<S, LPS_DEFERRED_TRANSACTION_TREE_ID, DEFERRED_TRANSACTION_TREE_HEIGHT>;
