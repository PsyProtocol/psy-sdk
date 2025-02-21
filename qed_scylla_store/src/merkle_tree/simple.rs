
use async_trait::async_trait;
use anyhow::Result;
use kvq::traits::{KVQPair, KVQSerializable};
use qed_store::models::kvq_merkle::key::KVQMerkleNodeKey;
use qed_store::traits::merkle_store::{MerkleNodeStoreReaderImmutableAsync, MerkleNodeStoreWriterImmutableAsync};
use scylla::batch::{Batch, BatchStatement};
use scylla::prepared_statement::PreparedStatement;
use scylla::transport::session::{CurrentDeserializationApi, GenericSession};
use scylla::{Session, SessionBuilder};
use std::env;
use std::marker::PhantomData;
use std::sync::Arc;
use std::usize::MAX;

use tokio::sync::Semaphore;

/* 


        let mut result: Vec<u8> = Vec::with_capacity(32);
        result.push(((TABLE_TYPE & 0xFF00) >> 8) as u8); // 1
        result.push((TABLE_TYPE & 0xFF) as u8); // 2
        result.push(self.tree_id); // 3
        result.extend_from_slice(&self.primary_id.to_be_bytes()); // 11
        result.extend_from_slice(&self.secondary_id.to_be_bytes()); // 15
        result.push(self.level); // 16
        result.extend_from_slice(&self.index.to_be_bytes()); // 24
        result.extend_from_slice(&self.checkpoint_id.to_be_bytes()); // 32
    pub tree_id: u8,
    pub primary_id: u64,
    pub secondary_id: u32,
    pub level: u8,
    pub index: u64,
    pub checkpoint_id: u64,
*/



const MAX_PREPARED_INSERT_BATCH_SIZE: usize = 128usize;
const MAX_SELECT_SIZE: usize = 128usize;
#[derive(Clone)]
pub struct ScyllaMerkleStore<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16>{
    pub session: Arc<GenericSession<CurrentDeserializationApi>>,
    pub insert_prepared: Arc<PreparedStatement>,
    pub select_latest_prepared: Arc<PreparedStatement>,
    pub select_at_or_before_checkpoint_prepared: Arc<PreparedStatement>,
    pub select_value_at_or_before_checkpoint_prepared: Arc<PreparedStatement>,
    pub dump_all_nodes_prepared: Arc<PreparedStatement>,
    pub insert_prepared_batches: [Arc<Batch>; MAX_PREPARED_INSERT_BATCH_SIZE],
    //pub insert_prepared_batches: [Arc<Batch>; MAX_PREPARED_INSERT_BATCH_SIZE],
    pub select_2: Arc<PreparedStatement>,
    pub _hash: PhantomData<Hash>,
}

impl<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> ScyllaMerkleStore<Hash, TABLE_TYPE> {
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
        let select_2 = Arc::new(
            session
                .prepare(format!(r#"SELECT node_index, node_level, node_value from {}.{} 
                WHERE tree_id=? AND primary_id=? AND secondary_id=? 
                AND node_level IN (?, ?) AND node_index IN  (?, ?) AND checkpoint_id <= ? PER PARTITION LIMIT 1
                "#, keyspace, table_name))
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
            select_2,
            _hash: PhantomData::default(),
        })




    }
    pub async fn get_two_in_same_tree(&self, key_a: &KVQMerkleNodeKey<TABLE_TYPE>, key_b: &KVQMerkleNodeKey<TABLE_TYPE> ) -> anyhow::Result<Vec<Option<Hash>>> {

        /*
        
        
                WHERE tree_id=? AND primary_id=? AND secondary_id=? 
                AND node_level IN (?, ?) AND node_index IN  (?, ?) AND checkpoint_id <= ? PER PARTITION LIMIT 1
                
                 */
        let result = self.session.execute_unpaged(&self.select_2, (
            
            key_a.tree_id as i16,
            key_a.primary_id as i64,
            key_a.secondary_id as i64,

            key_a.level as i16,
            key_b.level as i16,
            
            key_b.index as i64,
            key_a.index as i64,
            
            key_a.checkpoint_id as i64,
        )).await?;
        let res = result.into_rows_result()?;

        let mut final_result = [None; 2];

        for row in res.rows::<(i64, i16, Vec<u8>)>()? {
            match row {
                Ok(a) => {
                    let (index, level, value) = a;
                    if index as u64 == key_a.index {
                        final_result[0] = Some(Hash::from_bytes(&value)?);
                    } else if index as u64 == key_b.index {
                        final_result[1] = Some(Hash::from_bytes(&value)?);
                    }else{
                        println!("weird");
                    }
                },
                Err(e) => println!("derser: {:?}",e),
            }

        }

        Ok(final_result.to_vec())
    }
    pub async fn get_many_values(&self, keys: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Vec<Option<Hash>>> {
        if KVQMerkleNodeKey::node_list_in_same_tree(keys) && false {
            // todo implement that
            todo!("implement this opt");
        }else{
            let mut results = Vec::with_capacity(keys.len());
            for key in keys.iter() {
                let v = self.get_node_value_at_checkpoint(key).await?;
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
            for node in nodes_iter{
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
            self.session.batch(&self.insert_prepared_batches[remainder_nodes-1], row).await?;

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
    pub async fn get_latest_node(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
        let v = self.session.execute_unpaged(&self.select_latest_prepared, (
            key.tree_id as i16,
            key.primary_id as i64,
            key.secondary_id as i64,
            key.level as i16,
            key.index as i64,
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(i64,Vec<u8>)>()?;
        match res {
            Some(r) =>  Ok(Some(KVQPair {
                key: KVQMerkleNodeKey::<TABLE_TYPE> {
                    tree_id: key.tree_id,
                    primary_id: key.primary_id,
                    secondary_id: key.secondary_id,
                    level: key.level,
                    index: key.index,
                    checkpoint_id: r.0 as u64,
                },
                value: Hash::from_bytes(&r.1)?,
            })),
            None =>  Ok(None),
        }
    }

    pub async fn get_node_value_at_checkpoint(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<Hash>> {
        let v = self.session.execute_unpaged(&self.select_value_at_or_before_checkpoint_prepared, (
            key.tree_id as i16,
            key.primary_id as i64,
            key.secondary_id as i64,
            key.level as i16,
            key.index as i64,
            key.checkpoint_id as i64
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(Vec<u8>,)>()?;

        match res {
            Some(r) =>  Ok(Some(Hash::from_bytes(&r.0)?)),
            None =>  Ok(None),
        }
    }


    pub async fn get_node_at_checkpoint(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
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
            Some(r) =>  Ok(Some(KVQPair {
                key: KVQMerkleNodeKey::<TABLE_TYPE> {
                    tree_id: key.tree_id,
                    primary_id: key.primary_id,
                    secondary_id: key.secondary_id,
                    level: key.level,
                    index: key.index,
                    checkpoint_id: r.0 as u64,
                    
                },
                value: Hash::from_bytes(&r.1)?,
            })),
            None =>  Ok(None),
        }
    }
}


#[async_trait]
impl<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> MerkleNodeStoreReaderImmutableAsync<Hash, TABLE_TYPE> for ScyllaMerkleStore<Hash, TABLE_TYPE> {
    async fn get_node_if_exists(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
        Ok(self.get_node_at_checkpoint(key).await?)
    }
    async fn get_node_value_if_exists(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<Hash>> {
        Ok(self.get_node_value_at_checkpoint(key).await?)
    }
    async fn get_node_values(&self, keys: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Vec<Option<Hash>>> {
        Ok(self.get_many_values(keys).await?)

    }
}
#[async_trait]
impl<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> MerkleNodeStoreWriterImmutableAsync<Hash, TABLE_TYPE> for ScyllaMerkleStore<Hash, TABLE_TYPE> {
    async fn set_node_params(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>, value: Hash) -> anyhow::Result<()> {
        self.insert_node(key, value).await
    }
    async fn set_nodes(&self, nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>]) -> anyhow::Result<()> {
        self.insert_many_nodes(nodes).await

    }
    async fn set_nodes_same_tree(&self, nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>]) -> anyhow::Result<()> {
        self.insert_many_nodes(nodes).await
    }
}