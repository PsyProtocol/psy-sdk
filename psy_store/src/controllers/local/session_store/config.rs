use kvq::adapters::standard::KVQStandardAdapter;
use psy_core::config::network_constants::DEFERRED_TRANSACTION_TREE_HEIGHT;
use psy_data::{
    config::store_config::{PsyHash, PsyHasher},
    models::kvq_merkle::{key::KVQMerkleNodeKey, model::KVQFixedConfigMerkleTreeModel},
};

pub const LOCAL_PROVING_SESSION_TREE_TABLE_TYPE: u16 = 0xFE01;

pub type LocalProvingSessionTreeStore<
    S,
    const TREE_ID: u8,
    const HEIGHT: u8,
    IDKVA = KVQStandardAdapter<S, KVQMerkleNodeKey<LOCAL_PROVING_SESSION_TREE_TABLE_TYPE>, PsyHash>,
> = KVQFixedConfigMerkleTreeModel<TREE_ID, HEIGHT, 0, 0, LOCAL_PROVING_SESSION_TREE_TABLE_TYPE, false, S, IDKVA, PsyHash, PsyHasher>;

pub const LPS_DEFERRED_TRANSACTION_TREE_ID: u8 = 1;

pub type LPSDeferredTransactionTreeStore<S, IDKVA = KVQStandardAdapter<S, KVQMerkleNodeKey<LOCAL_PROVING_SESSION_TREE_TABLE_TYPE>, PsyHash>> =
    LocalProvingSessionTreeStore<S, LPS_DEFERRED_TRANSACTION_TREE_ID, DEFERRED_TRANSACTION_TREE_HEIGHT, IDKVA>;
