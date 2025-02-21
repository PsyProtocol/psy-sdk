use std::sync::Arc;

use kvq::memory::{immutable::KVQImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore};
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_scylla_store::merkle_tree::{ScyllaMerkleStore, ScyllaMerkleStorePerf1};
use qed_store::{config::store_config::{CheckpointTreeStore, ProtocolTreeStore, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE}, models::kvq_merkle::{key::KVQMerkleNodeKey, model::{KVQFixedConfigMerkleTreeModelCoreImmutable, KVQMerkleTreeModelCoreImmutable}}, traits::merkle_store::{QMerkleTreeModel, QMerkleTreeModelCoreImmutableAsync}};
use scylla::{transport::session::{CurrentDeserializationApi, GenericSession}, Session, SessionBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);

    const TABLE_TYPE: u16 = PROTOCOL_TREE_TABLE_TYPE;
    let sms = ScyllaMerkleStorePerf1::<QEDHash, PROTOCOL_TREE_TABLE_TYPE>::init("examples_ks".to_string(), "merkle_store_f".to_string(), session).await?;
    type QMerkleStore = QMerkleTreeModel<ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE, false>;




    let tree_height = 11;

    timer.lap("start 10000 set leaf scylladb");

    for i in 0..1000 {
        QMerkleStore::set_leaf(&sms, &KVQMerkleNodeKey::new_simple(2, tree_height, i, i), &QHashOut::rand()).await?;
    }
    timer.lap("finished 10000 set leaf scylladb");

    type MS = KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore>;
    let st: KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore> = KVQImmutableStoreWrapper::<KVQSimpleMemoryBackingStore>::new(
        KVQSimpleMemoryBackingStore::new(),
    );
    timer.lap("start 10000 set leaf mem");

    for i in 0..10000 {
        ProtocolTreeStore::<MS, 12, 32>::set_leaf_fc_imm(&st, 0, i as u64, QHashOut::rand())?;
        //KVQMerkleTreeModelCoreImmutable::<TABLE_TYPE, false, _, , QEDHash, QEDHasher>::set_leaf(&st, &KVQMerkleNodeKey::new_simple(2, tree_height, i, 0), QHashOut::rand())?;
    }
    
    timer.lap("finished 10000 set leaf memory");


    let all_debug_nodes = sms.dump_all_nodes_debug().await?;
    println!("all_debug_nodes.len = {}",all_debug_nodes.len());

    Ok(())
}