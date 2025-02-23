use std::sync::Arc;

use kvq::{memory::{immutable::KVQImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore}, traits::KVQPair};
use plonky2::{field::goldilocks_field::GoldilocksField, util::{log2_ceil, log2_strict}};
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_crypto::hash::traits::hasher::FieldQHasher;
use qed_scylla_store::merkle_tree::{ScyllaMerkleStore, ScyllaMerkleStorePerf1};
use qed_store::{config::store_config::{ProtocolTreeStore, QEDFelt, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE}, models::kvq_merkle::{key::KVQMerkleNodeKey, model::{KVQFixedConfigMerkleTreeModelCoreImmutable, KVQFixedConfigMerkleTreeModelReaderCore}}, traits::merkle_store::{QEDMerkleTreeModelReaderCoreAsync, QMerkleTreeModel, QMerkleTreeModelCoreImmutableAsync}};
use scylla::{transport::session::{CurrentDeserializationApi, GenericSession}, Session, SessionBuilder};
use tokio::task::JoinHandle;

/* 
use async_trait::async_trait;
use anyhow::Result;
use kvq::traits::{KVQPair, KVQSerializable};
use qed_core::data::qhashout::QHashOut;
use qed_core::utils::debug_timer::DebugTimer;
use qed_crypto::hash::traits::hasher::{MerkleZeroHasherWithCacheMarkedLeaf, MerkleZeroHasherWithMarkedLeaf};
use qed_store::config::store_config::{QEDHash, QEDHasher};
use qed_store::models::kvq_merkle::key::KVQMerkleNodeKey;
use scylla::batch::{Batch, BatchStatement};
use scylla::prepared_statement::PreparedStatement;
use scylla::transport::session::{CurrentDeserializationApi, GenericSession};
use scylla::{Session, SessionBuilder};
use std::env;
use std::marker::PhantomData;
use std::sync::Arc;
use std::usize::MAX;

use tokio::sync::Semaphore;

pub struct TreeId {
    tree_and_primary_id: i64,
    secondary_id: i64,
}
const MAX_PREPARED_INSERT_BATCH_SIZE: usize = 64usize;
#[derive(Clone)]
pub struct ScyllaMerkleStore<Hash: Copy + Clone + Send + Sync + KVQSerializable, Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>, const TABLE_TYPE: u16>{
    pub session: Arc<GenericSession<CurrentDeserializationApi>>,
    pub insert_prepared: Arc<PreparedStatement>,
    pub select_latest_prepared: Arc<PreparedStatement>,
    pub select_at_or_before_checkpoint_prepared: Arc<PreparedStatement>,
    pub select_value_at_or_before_checkpoint_prepared: Arc<PreparedStatement>,
    pub dump_all_nodes_prepared: Arc<PreparedStatement>,
    pub insert_prepared_batches: [Arc<Batch>; MAX_PREPARED_INSERT_BATCH_SIZE],
    pub _hash: PhantomData<Hash>,
    pub _hasher: PhantomData<Hasher>,
}

impl<Hash: Copy + Clone + Send + Sync + KVQSerializable, Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>, const TABLE_TYPE: u16> ScyllaMerkleStore<Hash, Hasher, TABLE_TYPE> {
    pub async fn init(keyspace: String, table_name: String, session: Arc<GenericSession<CurrentDeserializationApi>>) -> anyhow::Result<Self> {

        session.query_unpaged(format!("CREATE KEYSPACE IF NOT EXISTS {} WITH REPLICATION = {{'class' : 'NetworkTopologyStrategy', 'replication_factor' : 1}}", keyspace), &[]).await?;
    
        println!("created key space");
        let q= format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.{} (
                tree_id smallint,
                primary_id bigint,
                secondary_id bigint,
                node_level smallint,
                node_index bigint,
                checkpoint_id bigint,
                node_value blob,
                PRIMARY KEY ((tree_id, primary_id, secondary_id, node_level, node_index), checkpoint_id)
            ) WITH CLUSTERING ORDER BY (checkpoint_id DESC)
            "#,
            keyspace, table_name
        );
        println!("q: {}",q);
        session
            .query_unpaged(
                q,
                &[],
            )
            .await?;
    println!("created tbl");
    let insert_prepared = Arc::new(
        session
            .prepare(format!("INSERT INTO {}.{} (tree_id, primary_id, secondary_id, node_level, node_index, checkpoint_id, node_value) VALUES (?, ?, ?, ?, ?, ?, ?)", keyspace, table_name))
            .await?,
    );

    let mut batches = Vec::with_capacity(MAX_PREPARED_INSERT_BATCH_SIZE);
    
    let insert_prepared_alt = 
        session
            .prepare(format!("INSERT INTO {}.{} (tree_id, primary_id, secondary_id, node_level, node_index, checkpoint_id, node_value) VALUES (?, ?, ?, ?, ?, ?, ?)", keyspace, table_name))
            .await?;
    for i in 0..MAX_PREPARED_INSERT_BATCH_SIZE {
        let mut batch: Batch = Default::default();
        for _ in 0..=i {
            batch.append_statement(insert_prepared_alt.clone());
        }
        let prepared_batch = session.prepare_batch(&batch).await?;
        batches.push(Arc::new(prepared_batch));
    }

    let insert_prepared_batches: [Arc<Batch>; MAX_PREPARED_INSERT_BATCH_SIZE] = match batches.try_into() {
        Ok(x) => x,
        Err(_) => anyhow::bail!("error preparing batches"),
    };
    
            let select_latest_prepared = Arc::new(
                session
                    .prepare(format!("SELECT checkpoint_id, node_value from {}.{} WHERE tree_id=? AND primary_id=? AND secondary_id=? AND node_level=? AND node_index=? LIMIT 1", keyspace, table_name))
                    .await?,
            );
    
            let select_at_or_before_checkpoint_prepared = Arc::new(
                session
                    .prepare(format!("SELECT checkpoint_id, node_value from {}.{} WHERE tree_id=? AND primary_id=? AND secondary_id=? AND node_level=? AND node_index=? AND checkpoint_id <= ? LIMIT 1", keyspace, table_name))
                    .await?,
            );
            let select_value_at_or_before_checkpoint_prepared = Arc::new(
                session
                    .prepare(format!("SELECT node_value from {}.{} WHERE tree_id=? AND primary_id=? AND secondary_id=? AND node_level=? AND node_index=? AND checkpoint_id <= ? LIMIT 1", keyspace, table_name))
                    .await?,
            );
            let dump_all_nodes_prepared = Arc::new(
                session
                    .prepare(format!("SELECT * from {}.{}", keyspace, table_name))
                    .await?,
            );

        Ok(Self {
            session,
            insert_prepared,
            select_latest_prepared,
            insert_prepared_batches,
            select_at_or_before_checkpoint_prepared,
            select_value_at_or_before_checkpoint_prepared,
            dump_all_nodes_prepared,
            _hash: PhantomData::default(),
            _hasher: PhantomData::default(),
        })




    }
    pub async fn get_many_values(&self,tree_height:u8, keys: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Vec<Hash>> {
        if KVQMerkleNodeKey::node_list_in_same_tree(keys) && false {
            // todo implement that
            todo!("implement this opt");
        }else{
            let mut results = Vec::with_capacity(keys.len());
            for key in keys.iter() {
                let v = self.get_node_value_at_checkpoint(tree_height, key).await?;
                results.push(v)
            }
            Ok(results)
        }
    }
    pub async fn insert_many_nodes(&self, nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>]) -> anyhow::Result<()> {

        if nodes.len() == 0 {
            return Ok(())
        }else if nodes.len() == 1 {
            return self.insert_node(&nodes[0].key, nodes[0].value).await;
        }
        let remainder_nodes = nodes.len()%MAX_PREPARED_INSERT_BATCH_SIZE;
        let full_batches = nodes.len()/MAX_PREPARED_INSERT_BATCH_SIZE;


        let mut nodes_iter = nodes.iter();


        for _ in 0..full_batches {
            let mut row = Vec::with_capacity(MAX_PREPARED_INSERT_BATCH_SIZE);
            for (_, node) in (0..MAX_PREPARED_INSERT_BATCH_SIZE).zip(&mut nodes_iter) {
                row.push((
                    node.key.tree_id as i16,
                    node.key.primary_id as i64,
                    node.key.secondary_id as i64,
                    node.key.level as i16,
                    node.key.index as i64,
                    node.key.checkpoint_id as i64,
                    node.value.to_bytes()?
                ));
            }
            self.session.batch(&self.insert_prepared_batches[MAX_PREPARED_INSERT_BATCH_SIZE-1], row).await?;
        }

        if remainder_nodes != 0 {
            let mut row = Vec::with_capacity(MAX_PREPARED_INSERT_BATCH_SIZE);
            for node in nodes.iter(){
                row.push((
                    node.key.tree_id as i16,
                    node.key.primary_id as i64,
                    node.key.secondary_id as i64,
                    node.key.level as i16,
                    node.key.index as i64,
                    node.key.checkpoint_id as i64,
                    node.value.to_bytes()?
                ));
            }
            self.session.batch(&self.insert_prepared_batches[remainder_nodes], row).await?;

        }


        //Vec::with_capacity(nodes.len());





        Ok(())
    }
    pub async fn insert_node(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>, value: Hash) -> anyhow::Result<()> {
        self.session.execute_unpaged(&self.insert_prepared, (
            key.tree_id as i16,
            key.primary_id as i64,
            key.secondary_id as i64,
            key.level as i16,
            key.index as i64,
            key.checkpoint_id as i64,
            value.to_bytes()?
        )).await?;

        Ok(())
    }
    pub async fn dump_all_nodes_debug(&self) -> anyhow::Result<Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
        let v = self.session.execute_unpaged(&self.dump_all_nodes_prepared, ()).await?;

        let res = v.into_rows_result()?;
        

        let mut results = Vec::new();
        for i in res.rows::<(i16,i64,i64,i16,i64,i64,Vec<u8>)>()? {
            let (tree_id, primary_id, secondary_id, level, index, checkpoint_id, value ) = i?;
            results.push(KVQPair {
                key: KVQMerkleNodeKey::<TABLE_TYPE>{
                    tree_id: tree_id as u8,
                    primary_id: primary_id as u64,
                    secondary_id: secondary_id as u32,
                    level: level as u8,
                    index: index as u64,
                    checkpoint_id: checkpoint_id as u64,
                },
                value: Hash::from_bytes(&value)?,
            });
        }

        Ok(results)




    }
    pub async fn get_latest_node(&self, tree_height: u8, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> {
        let v = self.session.execute_unpaged(&self.select_latest_prepared, (
            key.tree_id as i16,
            key.primary_id as i64,
            key.secondary_id as i64,
            key.level as i16,
            key.index as i64,
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(i64,Vec<u8>)>()?;
        match res {
            Some(r) =>  Ok(KVQPair {
                key: KVQMerkleNodeKey::<TABLE_TYPE> {
                    tree_id: key.tree_id,
                    primary_id: key.primary_id,
                    secondary_id: key.secondary_id,
                    level: key.level,
                    index: key.index,
                    checkpoint_id: r.0 as u64,
                },
                value: Hash::from_bytes(&r.1)?,
            }),
            None =>  Ok(KVQPair {
                key: key.to_owned(),
                value: Hasher::get_zero_hash((tree_height-key.level) as usize),
            }),
        }
    }

    pub async fn get_node_value_at_checkpoint(&self, tree_height: u8, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash> {
        let v = self.session.execute_unpaged(&self.select_value_at_or_before_checkpoint_prepared, (
            key.tree_id as i16,
            key.primary_id as i64,
            key.secondary_id as i64,
            key.level as i16,
            key.index as i64,
            key.checkpoint_id as i64
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(Vec<u8>)>()?;
        match res {
            Some(r) =>  Hash::from_bytes(&r.1),
            None =>  Ok(Hasher::get_zero_hash((tree_height-key.level) as usize)),
        }
    }


    pub async fn get_node_at_checkpoint(&self, tree_height: u8, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> {
        let v = self.session.execute_unpaged(&self.select_at_or_before_checkpoint_prepared, (
            key.tree_id as i16,
            key.primary_id as i64,
            key.secondary_id as i64,
            key.level as i16,
            key.index as i64,
            key.checkpoint_id as i64
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(i64,Vec<u8>)>()?;
        match res {
            Some(r) =>  Ok(KVQPair {
                key: KVQMerkleNodeKey::<TABLE_TYPE> {
                    tree_id: key.tree_id,
                    primary_id: key.primary_id,
                    secondary_id: key.secondary_id,
                    level: key.level,
                    index: key.index,
                    checkpoint_id: r.0 as u64,
                    
                },
                value: Hash::from_bytes(&r.1)?,
            }),
            None =>  Ok(KVQPair {
                key: key.to_owned(),
                value: Hasher::get_zero_hash((tree_height-key.level) as usize),
            }),
        }
    }
}
#[async_trait]
pub trait MerkleNodeStoreReaderImmutableAsync<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> {
    async fn get_node_latest(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;
    async fn get_node(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;
    async fn get_nodes(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;
    async fn get_nodes_same_tree(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;


    async fn get_node_value_latest(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash>;
    async fn get_node_value(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash>;
    async fn get_node_values(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Hash>;
    async fn get_node_values_same_tree(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Hash>;
}
#[async_trait]
pub trait MerkleNodeStoreWriterImmutableAsync<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> {
    async fn set_node_params(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>, value: Hash) -> anyhow::Result<Hash>;

    async fn set_node(&self, node: &KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>) -> anyhow::Result<Hash> {
        self.set_node_params(&node.key, node.value).await
    }
    async fn set_nodes(&self, nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>]) -> anyhow::Result<()>;
    async fn set_nodes_same_tree(&self, nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>]) -> anyhow::Result<()>;
}


*/
async fn run_insert_data() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);

    const TABLE_TYPE: u16 = 123;
    let sms = ScyllaMerkleStorePerf1::<QEDHash, TABLE_TYPE>::init("examples_ks".to_string(), "merk1le_stozre_z".to_string(), session).await?;

    let tree_height = 10;
    /* 
    let ex_node_key = KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 1);
    let ex_node_key2 = KVQMerkleNodeKey::<TABLE_TYPE>::new(0, 123, 123, 1, 1, 1);
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,1,1)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,9)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 3), QHashOut::from_values(3,3,3,3)).await?;
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,99,1)).await?;

    let big_15: [KVQMerkleNodeKey<TABLE_TYPE>; 15] = core::array::from_fn(|i|{
        KVQMerkleNodeKey::new(19, 22, 22, 32, i as u64, 2)
    });

    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    for k in big_15.iter() {
        sms.insert_node(k, QHashOut::rand()).await?;
    }
    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    */

/* 
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 0), QHashOut::from_values(5,5,5,5)).await?;
    timer.lap("start insert 100000");
    for i in 0..100000usize {
        sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, (i%20) as u8, i as u64, (i/5) as u64), QHashOut::from_values(3,3,3,3)).await?;
    }
    timer.lap("end insert 100000");*/


    let worker_count = 16usize;
    let items_per_worker = 20000usize;
    let checkpoint_id = 2u64;
    timer.event(format!("start insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));



    let res = (0..worker_count).map(|w_ind|{
        let dq = sms.clone();
        let start_idx = items_per_worker*w_ind;
        //let item_count = items_per_worker;
        let jhandle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

            let batch = (0..items_per_worker).map(|i|{
                let i = start_idx+i;
                KVQPair {
                    key: KVQMerkleNodeKey::<TABLE_TYPE>::new(23, 123, 123, (i%20) as u8, i as u64, checkpoint_id),
                    value: QHashOut::<GoldilocksField>::rand(),
                }
            }).collect::<Vec<_>>();
            dq.insert_many_nodes(&batch).await?;
    
            Ok(())
        });
        jhandle
    }).collect::<Vec<_>>();

    for r in res {
        r.await??;
    }


    timer.event(format!("end insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));



    Ok(())
}
async fn run_insert_tree_g() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);

    let sms = ScyllaMerkleStorePerf1::<QEDHash, PROTOCOL_TREE_TABLE_TYPE>::init("examples_ks".to_string(), "merkle_store_f".to_string(), session).await?;
    type QMerkleStore = QMerkleTreeModel<ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE, false>;
    const TABLE_TYPE: u16 = PROTOCOL_TREE_TABLE_TYPE;

    /* 
    let ex_node_key = KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 1);
    let ex_node_key2 = KVQMerkleNodeKey::<TABLE_TYPE>::new(0, 123, 123, 1, 1, 1);
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,1,1)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,9)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 3), QHashOut::from_values(3,3,3,3)).await?;
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,99,1)).await?;

    let big_15: [KVQMerkleNodeKey<TABLE_TYPE>; 15] = core::array::from_fn(|i|{
        KVQMerkleNodeKey::new(19, 22, 22, 32, i as u64, 2)
    });

    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    for k in big_15.iter() {
        sms.insert_node(k, QHashOut::rand()).await?;
    }
    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    */

/* 
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 0), QHashOut::from_values(5,5,5,5)).await?;
    timer.lap("start insert 100000");
    for i in 0..100000usize {
        sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, (i%20) as u8, i as u64, (i/5) as u64), QHashOut::from_values(3,3,3,3)).await?;
    }
    timer.lap("end insert 100000");*/


    const TREE_HEIGHT: u8 = 2;
    let worker_count = 2usize;
    let items_per_worker = (1usize<<(TREE_HEIGHT as usize))/worker_count;
    let root_level = log2_strict(items_per_worker) as u8;
    println!("root_level: {}",root_level);
    let checkpoint_id = 2u64;
    let tree_id = 112u8;
    let mut recreated_pairs = (0..(items_per_worker*worker_count)).map(|i| {
        let w_ind = i/items_per_worker;
        let x = i%items_per_worker;
        let start_idx = w_ind*items_per_worker;
        KVQPair{key: KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(tree_id, TREE_HEIGHT, (start_idx + x) as u64, checkpoint_id), value: QHashOut::<QEDFelt>::from_values(1337, w_ind as u64, x as u64, (start_idx+x)  as u64)}
    }).collect::<Vec<_>>();


    type MS = KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore>;
    let st: KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore> = KVQImmutableStoreWrapper::<KVQSimpleMemoryBackingStore>::new(
        KVQSimpleMemoryBackingStore::new(),
    );
    timer.lap("start 10000 set leaf mem");

    for rc in recreated_pairs.iter() {
        let dmp_a = ProtocolTreeStore::<MS, 12, TREE_HEIGHT>::set_leaf_fc_imm(&st,rc.key.checkpoint_id, rc.key.index, rc.value)?;
        let dmp_b = QMerkleStore::set_leaf(&sms, &rc.key, &rc.value).await?;
        //println!("\n\nmem:\n{}\nscyl:\n{}\n\n", serde_json::to_string(&dmp_a).unwrap(),serde_json::to_string(&dmp_b).unwrap());
        //assert_eq!(dmp_a, dmp_b, "mp not the same")
        println!("\n\nmem:\n{:#?}\n\nscyl:\n{:#?}\n\n", &dmp_a,&dmp_b);
        
        //KVQMerkleTreeModelCoreImmutable::<TABLE_TYPE, false, _, , QEDHash, QEDHasher>::set_leaf(&st, &KVQMerkleNodeKey::new_simple(2, tree_height, i, 0), QHashOut::rand())?;
    }
    let dat_root = ProtocolTreeStore::<MS, 12, TREE_HEIGHT>::get_root_fc(&st, checkpoint_id)?;
    println!("got dat root: {:?}",dat_root);

    let root_left= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 1 ,0, checkpoint_id)).await?;

    let root_right= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 1 ,1, checkpoint_id)).await?;

    let compa = QEDHasher::q_two_to_one(root_left, root_right);
    println!("compa: {:?}, l={:?}, r={:?}",compa,root_left, root_right);
    //assert_eq!(dat_root, compa, "mp not the same");
    //QMerkleStore::rehash_sub_tree_top(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 7, 0, checkpoint_id)).await?;


let root2= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 0 ,0, checkpoint_id)).await?;
//assert_eq!(dat_root, root2, "mp not the same");

println!("root2a: {:?}",root2);
    
    
    timer.lap("finished 10000 set leaf memory");





    timer.event(format!("start insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));

    let mut left_nodes = recreated_pairs[0..items_per_worker].to_vec();

    let nca_left = QMerkleStore::smart_injest_nca(&sms, TREE_HEIGHT as usize, root_level, &mut left_nodes).await?;
    let mut right_nodes = recreated_pairs[items_per_worker..].to_vec();

    let nca_right = QMerkleStore::smart_injest_nca(&sms, TREE_HEIGHT as usize, root_level, &mut right_nodes).await?;

    //println!("nca: {}", serde_json::to_string(&nca_left).unwrap());

    println!("nca: {:?}", nca_left);
    QMerkleStore::rehash_sub_tree_top(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, TREE_HEIGHT-1, 0, checkpoint_id)).await?;



    let root_left= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 1 ,0, checkpoint_id)).await?;

    let root_right= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 1 ,1, checkpoint_id)).await?;

    let comp = QEDHasher::q_two_to_one(root_left, root_right);
    println!("comp: {:?}, l={:?}, r={:?}",comp,root_left, root_right);

    QMerkleStore::rehash_sub_tree_top(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, TREE_HEIGHT-1, 0, checkpoint_id)).await?;


let root2= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 0 ,0, checkpoint_id)).await?;
println!("root2: {:?}",root2);
    

/* 
    let res = (0..worker_count).map(|w_ind|{
        let dq = sms.clone();
        let recreated_pairs = recreated_pairs.clone();
        let start_idx = items_per_worker*w_ind;
        //let item_count = items_per_worker;
        let jhandle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

            let mut pairs = (0..items_per_worker).map(|x| {
                KVQPair{key: KVQMerkleNodeKey::new_simple(2, TREE_HEIGHT, (start_idx + x) as u64, checkpoint_id), value: QHashOut::from_values(1337, w_ind as u64, x as u64, (start_idx+x) as u64)}
            }).collect::<Vec<_>>();
            if !pairs.eq(&recreated_pairs[(start_idx)..(start_idx+items_per_worker)]){
                println!("bad pairs");
            }
            QMerkleStore::smart_injest_nca(&dq, TREE_HEIGHT as usize, root_level, &mut pairs).await?;

            Ok(())
        });
        jhandle
    }).collect::<Vec<_>>();

    for r in res {
        r.await??;
    }


QMerkleStore::rehash_sub_tree(&sms, (TREE_HEIGHT-root_level) as usize, &KVQMerkleNodeKey::new_simple(2, 0, 0, checkpoint_id)).await?;

*/


let root2= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 0 ,0, checkpoint_id)).await?;

println!("got new root11: {:?}",root2);
    timer.event(format!("end insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));



    Ok(())
}
async fn run_insert_tree_g2() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);

    let sms = ScyllaMerkleStorePerf1::<QEDHash, PROTOCOL_TREE_TABLE_TYPE>::init("examples_ks".to_string(), "merkle_store_f".to_string(), session).await?;
    type QMerkleStore = QMerkleTreeModel<ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE, false>;
    const TABLE_TYPE: u16 = PROTOCOL_TREE_TABLE_TYPE;

    /* 
    let ex_node_key = KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 1);
    let ex_node_key2 = KVQMerkleNodeKey::<TABLE_TYPE>::new(0, 123, 123, 1, 1, 1);
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,1,1)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,9)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 3), QHashOut::from_values(3,3,3,3)).await?;
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,99,1)).await?;

    let big_15: [KVQMerkleNodeKey<TABLE_TYPE>; 15] = core::array::from_fn(|i|{
        KVQMerkleNodeKey::new(19, 22, 22, 32, i as u64, 2)
    });

    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    for k in big_15.iter() {
        sms.insert_node(k, QHashOut::rand()).await?;
    }
    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    */

/* 
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 0), QHashOut::from_values(5,5,5,5)).await?;
    timer.lap("start insert 100000");
    for i in 0..100000usize {
        sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, (i%20) as u8, i as u64, (i/5) as u64), QHashOut::from_values(3,3,3,3)).await?;
    }
    timer.lap("end insert 100000");*/


    const TREE_HEIGHT: u8 = 8;
    let worker_count = 2usize;
    let items_per_worker = (1usize<<(TREE_HEIGHT as usize))/worker_count;
    let root_level = log2_strict(items_per_worker) as u8;
    let checkpoint_id = 2u64;
    let tree_id = 111u8;
    let mut recreated_pairs = (0..(items_per_worker*worker_count)).map(|i| {
        let w_ind = i/items_per_worker;
        let x = i%items_per_worker;
        let start_idx = w_ind*items_per_worker;
        KVQPair{key: KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(tree_id, TREE_HEIGHT, (start_idx + x) as u64, checkpoint_id), value: QHashOut::<QEDFelt>::from_values(1337, w_ind as u64, x as u64, (start_idx+x)  as u64)}
    }).collect::<Vec<_>>();


    type MS = KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore>;
    let st: KVQImmutableStoreWrapper<KVQSimpleMemoryBackingStore> = KVQImmutableStoreWrapper::<KVQSimpleMemoryBackingStore>::new(
        KVQSimpleMemoryBackingStore::new(),
    );
    timer.lap("start 10000 set leaf mem");

    for rc in recreated_pairs.iter() {
        let dmp_a = ProtocolTreeStore::<MS, 12, TREE_HEIGHT>::set_leaf_fc_imm(&st,rc.key.checkpoint_id, rc.key.index, rc.value)?;
        //let dmp_b = QMerkleStore::set_leaf(&sms, &rc.key, &rc.value).await?;
        //println!("\n\nmem:\n{}\nscyl:\n{}\n\n", serde_json::to_string(&dmp_a).unwrap(),serde_json::to_string(&dmp_b).unwrap());
        //assert_eq!(dmp_a, dmp_b, "mp not the same")

        //KVQMerkleTreeModelCoreImmutable::<TABLE_TYPE, false, _, , QEDHash, QEDHasher>::set_leaf(&st, &KVQMerkleNodeKey::new_simple(2, tree_height, i, 0), QHashOut::rand())?;
    }
    let dat_root = ProtocolTreeStore::<MS, 12, TREE_HEIGHT>::get_root_fc(&st, checkpoint_id)?;
    println!("got dat root: {:?}",dat_root);

    
    timer.lap("finished 10000 set leaf memory");



    timer.event(format!("start insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));

    let res = (0..worker_count).map(|w_ind|{
        let dq = sms.clone();
        let start_idx = items_per_worker*w_ind;
        //let item_count = items_per_worker;
        let jhandle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

            let mut pairs = (0..items_per_worker).map(|x| {
                KVQPair{key: KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(tree_id, TREE_HEIGHT, (start_idx + x) as u64, checkpoint_id), value: QHashOut::<QEDFelt>::from_values(1337, w_ind as u64, x as u64, (start_idx+x)  as u64)}

            }).collect::<Vec<_>>();
            QMerkleStore::smart_injest_nca(&dq, TREE_HEIGHT as usize, root_level, &mut pairs).await?;

            Ok(())
        });
        jhandle
    }).collect::<Vec<_>>();

    for r in res {
        r.await??;
    }




    let root_left= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 1 ,0, checkpoint_id)).await?;

    let root_right= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 1 ,1, checkpoint_id)).await?;

    let comp = QEDHasher::q_two_to_one(root_left, root_right);
    println!("comp: {:?}",comp);

    QMerkleStore::rehash_sub_tree(&sms, (TREE_HEIGHT-root_level) as usize, &KVQMerkleNodeKey::new_simple(tree_id, 0, 0, checkpoint_id)).await?;


let root2= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 0 ,0, checkpoint_id)).await?;
println!("root2: {:?}",root2);
    

/* 
    let res = (0..worker_count).map(|w_ind|{
        let dq = sms.clone();
        let recreated_pairs = recreated_pairs.clone();
        let start_idx = items_per_worker*w_ind;
        //let item_count = items_per_worker;
        let jhandle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

            let mut pairs = (0..items_per_worker).map(|x| {
                KVQPair{key: KVQMerkleNodeKey::new_simple(2, TREE_HEIGHT, (start_idx + x) as u64, checkpoint_id), value: QHashOut::from_values(1337, w_ind as u64, x as u64, (start_idx+x) as u64)}
            }).collect::<Vec<_>>();
            if !pairs.eq(&recreated_pairs[(start_idx)..(start_idx+items_per_worker)]){
                println!("bad pairs");
            }
            QMerkleStore::smart_injest_nca(&dq, TREE_HEIGHT as usize, root_level, &mut pairs).await?;

            Ok(())
        });
        jhandle
    }).collect::<Vec<_>>();

    for r in res {
        r.await??;
    }


QMerkleStore::rehash_sub_tree(&sms, (TREE_HEIGHT-root_level) as usize, &KVQMerkleNodeKey::new_simple(2, 0, 0, checkpoint_id)).await?;

*/


let root2= QMerkleStore::get_node(&sms, TREE_HEIGHT as usize, &KVQMerkleNodeKey::new_simple(tree_id, 0 ,0, checkpoint_id)).await?;

println!("got new root11: {:?}",root2);
    timer.event(format!("end insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));



    Ok(())
}

async fn run_insert_tree() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);

    let sms = ScyllaMerkleStorePerf1::<QEDHash, PROTOCOL_TREE_TABLE_TYPE>::init("examples_ks".to_string(), "merkle_store_f".to_string(), session).await?;
    type QMerkleStore = QMerkleTreeModel<ScyllaMerkleStorePerf1<QEDHash, PROTOCOL_TREE_TABLE_TYPE>, QEDHash, QEDHasher, PROTOCOL_TREE_TABLE_TYPE, false>;
    const TABLE_TYPE: u16 = PROTOCOL_TREE_TABLE_TYPE;

    /* 
    let ex_node_key = KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 1);
    let ex_node_key2 = KVQMerkleNodeKey::<TABLE_TYPE>::new(0, 123, 123, 1, 1, 1);
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,1,1)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,9)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 2), QHashOut::from_values(2,2,2,2)).await?;
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 3), QHashOut::from_values(3,3,3,3)).await?;
    sms.insert_node(&ex_node_key, QHashOut::from_values(1,1,99,1)).await?;

    let big_15: [KVQMerkleNodeKey<TABLE_TYPE>; 15] = core::array::from_fn(|i|{
        KVQMerkleNodeKey::new(19, 22, 22, 32, i as u64, 2)
    });

    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    for k in big_15.iter() {
        sms.insert_node(k, QHashOut::rand()).await?;
    }
    let res_15 = sms.go_sel_15(big_15).await?;
    println!("res_15: {:?}",res_15);
    */

/* 
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 0), QHashOut::from_values(5,5,5,5)).await?;
    timer.lap("start insert 100000");
    for i in 0..100000usize {
        sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, (i%20) as u8, i as u64, (i/5) as u64), QHashOut::from_values(3,3,3,3)).await?;
    }
    timer.lap("end insert 100000");*/


    let tree_height = 18;
    let worker_count = 2usize;
    let items_per_worker = (1usize<<(tree_height as usize))/worker_count;
    let root_level = log2_strict(items_per_worker) as u8;
    let checkpoint_id = 2u64;
    timer.event(format!("start insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));

    let mut recreated_pairs = (0..(items_per_worker*worker_count)).map(|i| {
        let w_ind = i/items_per_worker;
        let x = i%items_per_worker;
        let start_idx = w_ind*items_per_worker;
        KVQPair{key: KVQMerkleNodeKey::<TABLE_TYPE>::new_simple(2, tree_height, (start_idx + x) as u64, checkpoint_id), value: QHashOut::<QEDFelt>::from_values(1337, w_ind as u64, x as u64, (start_idx+x)  as u64)}
    }).collect::<Vec<_>>();


    let res = (0..worker_count).map(|w_ind|{
        let dq = sms.clone();
        let recreated_pairs = recreated_pairs.clone();
        let start_idx = items_per_worker*w_ind;
        //let item_count = items_per_worker;
        let jhandle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

            let mut pairs = (0..items_per_worker).map(|x| {
                KVQPair{key: KVQMerkleNodeKey::new_simple(2, tree_height, (start_idx + x) as u64, checkpoint_id), value: QHashOut::from_values(1337, w_ind as u64, x as u64, (start_idx+x) as u64)}
            }).collect::<Vec<_>>();
            if !pairs.eq(&recreated_pairs[(start_idx)..(start_idx+items_per_worker)]){
                println!("bad pairs");
            }
            QMerkleStore::smart_injest_nca(&dq, tree_height as usize, root_level, &mut pairs).await?;

            Ok(())
        });
        jhandle
    }).collect::<Vec<_>>();

    for r in res {
        r.await??;
    }


QMerkleStore::rehash_sub_tree(&sms, (tree_height-root_level) as usize, &KVQMerkleNodeKey::new_simple(2, 0, 0, checkpoint_id)).await?;

    timer.event(format!("end insert {}x{} = {}",items_per_worker,worker_count,items_per_worker*worker_count));



    Ok(())
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {

    run_insert_tree_g().await?;
    run_insert_data().await?;
    Ok(())
}