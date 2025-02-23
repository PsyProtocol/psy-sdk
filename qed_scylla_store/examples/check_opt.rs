use std::sync::Arc;

use kvq::{memory::{immutable::KVQImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore}, traits::KVQPair};
use plonky2::{field::goldilocks_field::GoldilocksField, util::log2_strict};
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_scylla_store::merkle_tree::{ScyllaMerkleStore, ScyllaMerkleStorePerf1};
use qed_store::{config::store_config::{CheckpointTreeStore, ProtocolTreeStore, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE}, models::kvq_merkle::{key::KVQMerkleNodeKey, model::{KVQFixedConfigMerkleTreeModelCoreImmutable, KVQFixedConfigMerkleTreeModelReaderCore, KVQMerkleTreeModelCoreImmutable}}, traits::merkle_store::{QEDMerkleTreeModelReaderCoreAsync, QMerkleTreeModel, QMerkleTreeModelCoreImmutableAsync}};
use rand::{thread_rng, RngCore};
use scylla::{transport::session::{CurrentDeserializationApi, GenericSession}, Session, SessionBuilder};
use tokio::task::JoinHandle;


type F = GoldilocksField;
type H = QEDHasher;
type MS = KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore>;
const TABLE_TYPE: u16 = PROTOCOL_TREE_TABLE_TYPE;
const TREE_HEIGHT: u8 = 14;
const TREE_ID: u8 = 100;
type MemTreeStore = ProtocolTreeStore<MS, TREE_ID, TREE_HEIGHT>;
type QMerkleStore = QMerkleTreeModel<ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE, false>;
type MerkleDBStore = ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>;

fn get_deterministic_node_for_worker(checkpoint_id: u64, global_index: usize, local_index: usize, worker_id: usize) -> KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, QEDHash>{


    KVQPair {
        key: KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(TREE_ID, TREE_HEIGHT as u8, global_index as u64, checkpoint_id),
        value: QHashOut::<F>::from_values(worker_id as u64, 1337u64, global_index as u64, local_index as u64),
    }
}
fn get_deterministic_node_for_index(checkpoint_id: u64, global_index: usize, items_per_worker: usize) -> KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, QEDHash>{
    let worker_id = global_index / items_per_worker;
    let local_index = global_index % items_per_worker;

    get_deterministic_node_for_worker(checkpoint_id, global_index, local_index, worker_id)
}    
async fn check_injest_full_tree(session: Arc<GenericSession<CurrentDeserializationApi>>) -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let rand_tbl = format!("mt_test_tbl_{}", thread_rng().next_u64());


    let sms = ScyllaMerkleStorePerf1::<QEDHash, PROTOCOL_TREE_TABLE_TYPE>::init("examples_ks".to_string(), rand_tbl, session).await?;


    let st = KVQImmutableStoreWrapper::<KVQSimpleMemoryBackingStore>::new(
        KVQSimpleMemoryBackingStore::new(),
    );

    let max_leaves = 1usize<<(TREE_HEIGHT as usize);
    let max_leaf_mask = (max_leaves-1) as u64;

    let worker_count = 32usize;
    let items_per_worker = (1usize<<(TREE_HEIGHT as usize))/worker_count;
    let root_level = log2_strict(items_per_worker) as u8;

    let checkpoint_id = 1;

    timer.event(format!("start {} insert leaf mem", max_leaves));

    for i in (0..max_leaves) {
        let leaf = get_deterministic_node_for_index(checkpoint_id, i, items_per_worker);

        MemTreeStore::set_leaf(&st, &leaf.key, leaf.value)?;
    }
    timer.event(format!("end {} insert leaf mem", max_leaves));

    let mem_root = MemTreeStore::get_root_fc(&st, checkpoint_id)?;
    println!("mem_root: {} ({:?})", serde_json::to_string(&mem_root).unwrap(), mem_root);




    timer.event(format!("start insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));

    let res = (0..worker_count).map(|worker_id|{
        let dq = sms.clone();
        let start_idx = items_per_worker*worker_id;
        
        //let item_count = items_per_worker;
        let jhandle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

            let mut pairs = (0..items_per_worker).map(|x| {
                let global_index = x + start_idx;
                let local_index= x;
                get_deterministic_node_for_worker(checkpoint_id, global_index, local_index, worker_id as usize)
            }).collect::<Vec<_>>();
            QMerkleStore::smart_injest_nca(&dq, TREE_HEIGHT as usize, root_level, &mut pairs).await?;

            Ok(())
        });
        jhandle
    }).collect::<Vec<_>>();

    for r in res {
        r.await??;
    }







    let first_node_at_rehash_level = get_deterministic_node_for_index(checkpoint_id, 0, items_per_worker).key.parent_at_level(root_level);

    QMerkleStore::rehash_sub_tree_top(&sms, TREE_HEIGHT as usize, &first_node_at_rehash_level).await?;
    timer.event(format!("end insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));

    assert_eq!(
        QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &first_node_at_rehash_level.root()).await?,
        MemTreeStore::get_root_fc(&st, checkpoint_id)?,
        "merkle store and mem tree store do not have the same roots"
    );

    Ok(())
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);
    
    check_injest_full_tree(session.clone()).await?;


    Ok(())
}