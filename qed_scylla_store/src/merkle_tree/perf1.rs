
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


pub fn get_partial_node_key<const TABLE_TYPE: u16>(bytes: &[u8]) -> anyhow::Result<KVQMerkleNodeKey<TABLE_TYPE>> {
    if bytes.len() != 22 {
        anyhow::bail!("error deserializing node uuid, invalid length");
    }
    Ok(KVQMerkleNodeKey {
        tree_id: bytes[0],
        primary_id: u64::from_be_bytes(bytes[1..9].try_into().unwrap()),
        secondary_id:  u32::from_be_bytes(bytes[9..13].try_into().unwrap()),
        level: bytes[13],
        index:  u64::from_be_bytes(bytes[14..22].try_into().unwrap()),
        checkpoint_id: 0,
    })
}
pub fn get_node_uuid<const TABLE_TYPE: u16>(key: &KVQMerkleNodeKey<TABLE_TYPE>) -> Vec<u8> {

    let mut result: Vec<u8> = Vec::with_capacity(22);
    result.push(key.tree_id); // 1
    result.extend_from_slice(&key.primary_id.to_be_bytes()); // 9
    result.extend_from_slice(&key.secondary_id.to_be_bytes()); // 13
    result.push(key.level); // 14
    result.extend_from_slice(&key.index.to_be_bytes()); // 22
    result
}
#[derive(Clone)]
pub struct ScyllaMerkleStorePerf1<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16>{
    pub session: Arc<GenericSession<CurrentDeserializationApi>>,
    pub insert_prepared: Arc<PreparedStatement>,
    pub select_latest_prepared: Arc<PreparedStatement>,
    pub select_at_or_before_checkpoint_prepared: Arc<PreparedStatement>,
    pub select_value_at_or_before_checkpoint_prepared: Arc<PreparedStatement>,
    pub dump_all_nodes_prepared: Arc<PreparedStatement>,
    pub insert_prepared_batches: [Arc<Batch>; MAX_PREPARED_INSERT_BATCH_SIZE],
    //pub insert_prepared_batches: [Arc<Batch>; MAX_PREPARED_INSERT_BATCH_SIZE],
    pub select_2: Arc<PreparedStatement>,
    pub select_15: Arc<PreparedStatement>,
    pub _hash: PhantomData<Hash>,
}

impl<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> ScyllaMerkleStorePerf1<Hash, TABLE_TYPE> {
    pub async fn init(keyspace: String, table_name: String, session: Arc<GenericSession<CurrentDeserializationApi>>) -> anyhow::Result<Self> {

        session.query_unpaged(format!("CREATE KEYSPACE IF NOT EXISTS {} WITH REPLICATION = {{'class' : 'NetworkTopologyStrategy', 'replication_factor' : 1}}", keyspace), &[]).await?;
    
        println!("created key space");
        let q= format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.{} (
                node_uuid blob,
                checkpoint_id bigint,
                node_value blob,
                PRIMARY KEY ((node_uuid), checkpoint_id)
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
            .prepare(format!("INSERT INTO {}.{} (node_uuid, checkpoint_id, node_value) VALUES (?, ?, ?)", keyspace, table_name))
            .await?,
    );

    let mut batches = Vec::with_capacity(MAX_PREPARED_INSERT_BATCH_SIZE);
    
    let insert_prepared_alt = 
        session
            .prepare(format!("INSERT INTO {}.{} (node_uuid, checkpoint_id, node_value) VALUES (?, ?, ?)", keyspace, table_name))
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
                .prepare(format!("SELECT checkpoint_id, node_value from {}.{} WHERE node_uuid=? LIMIT 1", keyspace, table_name))
                .await?,
        );

        let select_at_or_before_checkpoint_prepared = Arc::new(
            session
                .prepare(format!("SELECT checkpoint_id, node_value from {}.{} WHERE node_uuid=? AND checkpoint_id <= ? LIMIT 1", keyspace, table_name))
                .await?,
        );
        let select_value_at_or_before_checkpoint_prepared = Arc::new(
            session
                .prepare(format!("SELECT node_value from {}.{} WHERE node_uuid=? AND checkpoint_id <= ? LIMIT 1", keyspace, table_name))
                .await?,
        );
        let dump_all_nodes_prepared = Arc::new(
            session
                .prepare(format!("SELECT * from {}.{}", keyspace, table_name))
                .await?,
        );
        let select_2 = Arc::new(
            session
                .prepare(format!(r#"SELECT node_uuid, node_value from {}.{} 
                WHERE node_uuid IN (?, ?) AND checkpoint_id <= ? PER PARTITION LIMIT 1
                "#, keyspace, table_name))
                .await?,
        );
        let select_15 = Arc::new(
            session
                .prepare(format!(r#"SELECT node_uuid, node_value from {}.{} 
                WHERE node_uuid IN (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) AND checkpoint_id <= ? PER PARTITION LIMIT 1
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
            select_15,
            _hash: PhantomData::default(),
        })




    }
    pub async fn get_two_in_same_tree(&self, key_a: &KVQMerkleNodeKey<TABLE_TYPE>, key_b: &KVQMerkleNodeKey<TABLE_TYPE> ) -> anyhow::Result<Vec<Option<Hash>>> {

        /*
        
        
                WHERE tree_id=? AND primary_id=? AND secondary_id=? 
                AND node_level IN (?, ?) AND node_index IN  (?, ?) AND checkpoint_id <= ? PER PARTITION LIMIT 1
                
                 */

        let result = self.session.execute_unpaged(&self.select_2, (
            get_node_uuid(key_a),
            get_node_uuid(key_b),
            key_a.checkpoint_id as i64,
        )).await?;
        let res = result.into_rows_result()?;

        let mut final_result = [None; 2];

        for row in res.rows::<(Vec<u8>, Vec<u8>)>()? {
            match row {
                Ok(a) => {
                    let (node_uuid, value) = a;
                    let regen = get_partial_node_key(&node_uuid)?;
                    if regen.is_same_node_location(&key_a) {
                        final_result[0] = Some(Hash::from_bytes(&value)?);

                    }else if regen.is_same_node_location(key_b){
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


    pub async fn go_sel_15(&self, keys: [KVQMerkleNodeKey<TABLE_TYPE>; 15] ) -> anyhow::Result<Vec<Option<Hash>>> {

        /*
        
        
                WHERE tree_id=? AND primary_id=? AND secondary_id=? 
                AND node_level IN (?, ?) AND node_index IN  (?, ?) AND checkpoint_id <= ? PER PARTITION LIMIT 1
                
                 */

        let key_ids: [_; 15] = core::array::from_fn(|x| get_node_uuid(&keys[x]));


        



        let result = self.session.execute_unpaged(&self.select_15, (
            &key_ids[0],
            &key_ids[1],
            &key_ids[2],
            &key_ids[3],
            &key_ids[4],
            &key_ids[5],
            &key_ids[6],
            &key_ids[7],
            &key_ids[8],
            &key_ids[9],
            &key_ids[10],
            &key_ids[11],
            &key_ids[12],
            &key_ids[13],
            &key_ids[14],
            keys[0].checkpoint_id as i64,
        )).await?;
        let res = result.into_rows_result()?;

        let mut final_result = [None; 15];

        for row in res.rows::<(Vec<u8>, Vec<u8>)>()? {
            match row {
                Ok(a) => {
                    let (node_uuid, value) = a;
                    //let regen = get_partial_node_key(&node_uuid)?;

                    for (i, v) in key_ids.iter().enumerate(){
                        if v.eq(&node_uuid) {
                            final_result[i] = Some(Hash::from_bytes(&value)?);

                        }
                    }
                },
                Err(e) => println!("derser: {:?}",e),
            }

        }

        Ok(final_result.to_vec())
    }
    pub async fn get_many_values(&self, keys: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Vec<Option<Hash>>> {
        let mut results = Vec::with_capacity(keys.len());

        let full_batches = keys.len()/15;
        //let remainder = keys.len()%15;

        for batch_id in 0..full_batches {
            results.extend_from_slice(&self.go_sel_15(keys[(batch_id*15)..((batch_id+1)*15)].try_into().unwrap()).await?);

        }
        for key in keys[full_batches*15..].iter() {
            let v = self.get_node_value_at_checkpoint(key).await?;
            results.push(v)
        }
        Ok(results)
        
        /*
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
        }*/
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
                    get_node_uuid(&node.key),
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
                    get_node_uuid(&node.key),
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
            get_node_uuid(key),
            key.checkpoint_id as i64,
            value.to_bytes()?
        )).await?;

        Ok(())
    }
    pub async fn dump_all_nodes_debug(&self) -> anyhow::Result<Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
        let v = self.session.execute_unpaged(&self.dump_all_nodes_prepared, ()).await?;

        let res = v.into_rows_result()?;
        

        let mut results = Vec::new();
        for i in res.rows::<(Vec<u8>,i64,Vec<u8>)>()? {
            let (node_uuid, checkpoint_id, value ) = i?;

            let mut base_node = get_partial_node_key::<TABLE_TYPE>(&node_uuid)?;
            base_node.checkpoint_id = checkpoint_id as u64;

            results.push(KVQPair {
                key: base_node,
                value: Hash::from_bytes(&value)?,
            });
        }

        Ok(results)




    }
    pub async fn get_latest_node(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
        let v = self.session.execute_unpaged(&self.select_latest_prepared, (
            get_node_uuid(key),
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
            get_node_uuid(key),

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
            get_node_uuid(key),

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
impl<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> MerkleNodeStoreReaderImmutableAsync<Hash, TABLE_TYPE> for ScyllaMerkleStorePerf1<Hash, TABLE_TYPE> {
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
impl<Hash: Copy + Clone + Send + Sync + KVQSerializable, const TABLE_TYPE: u16> MerkleNodeStoreWriterImmutableAsync<Hash, TABLE_TYPE> for ScyllaMerkleStorePerf1<Hash, TABLE_TYPE> {
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