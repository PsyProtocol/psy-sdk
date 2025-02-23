use std::sync::Arc;

use kvq::traits::KVQPair;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_scylla_store::merkle_tree::{ScyllaMerkleStore, ScyllaMerkleStorePerf1};
use qed_store::{config::store_config::QEDHash, models::kvq_merkle::key::KVQMerkleNodeKey};
use scylla::{transport::session::{CurrentDeserializationApi, GenericSession}, Session, SessionBuilder};

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
#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");


    let session: Session = SessionBuilder::new().known_node("127.0.0.1:9042").build().await?;
    let session: Arc<GenericSession<CurrentDeserializationApi>> = Arc::new(session);

    const TABLE_TYPE: u16 = 123;
    let sms = ScyllaMerkleStorePerf1::<QEDHash, TABLE_TYPE>::init("examples_ks".to_string(), "merk1le_stozre_z".to_string(), session).await?;

    let tree_height = 10;
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

/* 
    sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, 1, 0, 0), QHashOut::from_values(5,5,5,5)).await?;
    timer.lap("start insert 100000");
    for i in 0..100000usize {
        sms.insert_node(&KVQMerkleNodeKey::new(0, 123, 123, (i%20) as u8, i as u64, (i/5) as u64), QHashOut::from_values(3,3,3,3)).await?;
    }
    timer.lap("end insert 100000");*/

    timer.lap("start insert 100000");
    let batch = (0..100000usize).map(|i|{
        KVQPair {
            key: KVQMerkleNodeKey::<TABLE_TYPE>::new(23, 123, 123, (i%20) as u8, i as u64, (i/5) as u64),
            value: QHashOut::<GoldilocksField>::rand(),
        }
    }).collect::<Vec<_>>();
    sms.insert_many_nodes(&batch).await?;
    timer.lap("end insert 100000");


    let res = sms.get_latest_node(&ex_node_key).await?;
    let res2 = sms.get_node_at_checkpoint(&ex_node_key).await?;

    println!("res: {:?}",res);
    println!("res2: {:?}",res2);
    let all_debug_nodes = sms.dump_all_nodes_debug().await?;
    println!("all_debug_nodes.len = {}",all_debug_nodes.len());

    Ok(())
}