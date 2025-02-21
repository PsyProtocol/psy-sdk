use std::sync::Arc;

use kvq::{memory::{immutable::KVQImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore}, traits::KVQPair};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_scylla_store::merkle_tree::{ScyllaMerkleStore, ScyllaMerkleStorePerf1};
use qed_store::{config::store_config::{CheckpointTreeStore, ProtocolTreeStore, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE}, models::kvq_merkle::{key::KVQMerkleNodeKey, model::{KVQFixedConfigMerkleTreeModelCoreImmutable, KVQMerkleTreeModelCoreImmutable}}, traits::merkle_store::{QMerkleTreeModel, QMerkleTreeModelCoreImmutableAsync}};
use rand::{thread_rng, RngCore};
use scylla::{transport::session::{CurrentDeserializationApi, GenericSession}, Session, SessionBuilder};


type F = GoldilocksField;
type H = QEDHasher;
type MS = KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore>;
const TABLE_TYPE: u16 = PROTOCOL_TREE_TABLE_TYPE;
const TREE_HEIGHT: u8 = 3;
const TREE_ID: u8 = 100;
type MemTreeStore = ProtocolTreeStore<MS, TREE_ID, TREE_HEIGHT>;
type QMerkleStore = QMerkleTreeModel<ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE, false>;
type MerkleDBStore = ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>;

    
async fn check_injest_leaf(session: Arc<GenericSession<CurrentDeserializationApi>>) -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let rand_tbl = format!("mt_test_tbl_{}", thread_rng().next_u64());


    let sms = ScyllaMerkleStorePerf1::<QEDHash, PROTOCOL_TREE_TABLE_TYPE>::init("examples_ks".to_string(), rand_tbl, session).await?;


    let st = KVQImmutableStoreWrapper::<KVQSimpleMemoryBackingStore>::new(
        KVQSimpleMemoryBackingStore::new(),
    );

    let max_leaves = 1usize<<(TREE_HEIGHT as usize);
    let max_leaf_mask = (max_leaves-1) as u64;

    let checkpoint_id = 1;


    let leaf_values = (0..(max_leaves/2)).map(|i|{
        KVQPair {
            key: KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(TREE_ID, TREE_HEIGHT as u8, i as u64, checkpoint_id),
            value: QHashOut::<F>::from_values(i as u64,i as u64,i as u64,i as u64),
        }
    }).collect::<Vec<_>>();

    timer.event(format!("start {} insert leaf scylladb and mem", max_leaves));

    for pair in leaf_values.iter() {
        let mem_dmp = MemTreeStore::set_leaf(&st, &pair.key, pair.value)?;
        let db_dmp = QMerkleStore::set_leaf(&sms, &pair.key, &pair.value).await?;
        assert_eq!(mem_dmp, db_dmp, "mem tree and merkle store disagree");
    }
    timer.event(format!("end {} insert leaf scylladb and mem", max_leaves));

    let first_node_at_rehash_level = KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(TREE_ID, TREE_HEIGHT-1, 0, checkpoint_id);
    
    let mut all_debug_nodes = sms.dump_all_nodes_debug().await?;
    all_debug_nodes.sort_by(|a,b| {
        if a.key.level != b.key.level {
            a.key.level.cmp(&b.key.level)
        }else if a.key.index != b.key.index {
            a.key.index.cmp(&b.key.index)
        }else{
            a.key.cmp(&b.key)
        }
    });

    QMerkleStore::rehash_sub_tree_top(&sms, TREE_HEIGHT as usize, &first_node_at_rehash_level).await?;


    let mut all_debug_nodes_2 = sms.dump_all_nodes_debug().await?;
    all_debug_nodes_2.sort_by(|a,b| {
        if a.key.level != b.key.level {
            a.key.level.cmp(&b.key.level)
        }else if a.key.index != b.key.index {
            a.key.index.cmp(&b.key.index)
        }else{
            a.key.cmp(&b.key)
        }
    });

    println!("all_dbg:\n\n{}\n\n",serde_json::to_string_pretty(&all_debug_nodes).unwrap());
    println!("all_dbg2:\n\n{}\n\n",serde_json::to_string_pretty(&all_debug_nodes_2).unwrap());
    println!("all_debug_nodes.len = {}",all_debug_nodes.len());
    println!("all_debug_nodes2.len = {}",all_debug_nodes_2.len());

    Ok(())
}


async fn check_basic_set_leaf(session: Arc<GenericSession<CurrentDeserializationApi>>) -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let rand_tbl = format!("mt_test_tbl_{}", thread_rng().next_u64());


    let sms = ScyllaMerkleStorePerf1::<QEDHash, PROTOCOL_TREE_TABLE_TYPE>::init("examples_ks".to_string(), rand_tbl, session).await?;

    let st = KVQImmutableStoreWrapper::<KVQSimpleMemoryBackingStore>::new(
        KVQSimpleMemoryBackingStore::new(),
    );

    let max_leaves = 1usize<<(TREE_HEIGHT as usize);
    let max_leaf_mask = (max_leaves-1) as u64;

    let checkpoint_id = 1;


    let leaf_values = (0..(max_leaves/2)).map(|i|{
        KVQPair {
            key: KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(TREE_ID, TREE_HEIGHT as u8, i as u64, checkpoint_id),
            value: QHashOut::<F>::from_values(i as u64,i as u64,i as u64,i as u64),
        }
    }).collect::<Vec<_>>();

    let mut leaf_vals = leaf_values.clone();

    timer.event(format!("start {} insert leaf scylladb and mem", max_leaves));

    for pair in leaf_values.iter() {
        let mem_dmp = MemTreeStore::set_leaf(&st, &pair.key, pair.value)?;
        let db_dmp = QMerkleStore::set_leaf(&sms, &pair.key, &pair.value).await?;
        assert_eq!(mem_dmp, db_dmp, "mem tree and merkle store disagree");
    }
    timer.event(format!("end {} insert leaf scylladb and mem", max_leaves));

    let first_node_at_rehash_level = KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(TREE_ID, TREE_HEIGHT-1, 0, checkpoint_id);
    
    let mut all_debug_nodes = sms.dump_all_nodes_debug().await?;
    all_debug_nodes.sort_by(|a,b| {
        if a.key.level != b.key.level {
            a.key.level.cmp(&b.key.level)
        }else if a.key.index != b.key.index {
            a.key.index.cmp(&b.key.index)
        }else{
            a.key.cmp(&b.key)
        }
    });
    QMerkleStore::smart_injest_nca(&sms, TREE_HEIGHT as usize, 1, &mut leaf_vals).await?;

    //QMerkleStore::rehash_sub_tree_top(&sms, TREE_HEIGHT as usize, &first_node_at_rehash_level).await?;


    let mut all_debug_nodes_2 = sms.dump_all_nodes_debug().await?;
    all_debug_nodes_2.sort_by(|a,b| {
        if a.key.level != b.key.level {
            a.key.level.cmp(&b.key.level)
        }else if a.key.index != b.key.index {
            a.key.index.cmp(&b.key.index)
        }else{
            a.key.cmp(&b.key)
        }
    });

    println!("all_dbg:\n\n{}\n\n",serde_json::to_string_pretty(&all_debug_nodes).unwrap());
    println!("all_dbg2:\n\n{}\n\n",serde_json::to_string_pretty(&all_debug_nodes_2).unwrap());
    println!("all_debug_nodes.len = {}",all_debug_nodes.len());
    println!("all_debug_nodes2.len = {}",all_debug_nodes_2.len());

    Ok(())
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);
    
    check_basic_set_leaf(session.clone()).await?;


    Ok(())
}