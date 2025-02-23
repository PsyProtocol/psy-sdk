
use async_trait::async_trait;
use anyhow::Result;
use kvq::traits::{KVQBinaryStoreReaderAsync, KVQBinaryStoreWriterImmutableAsync, KVQPair, KVQSerializable};
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

pub fn get_table_type(bytes: &[u8]) -> anyhow::Result<u16> {

    if bytes.len() < 2 {
        anyhow::bail!("missing table type");
    }
    Ok(((bytes[0] as u16)<<8) | (bytes[1] as u16))
}
pub fn chop_table_key(bytes: &[u8]) -> (Vec<u8>, u64) {
    if bytes.len() < 10 {
        (bytes.to_vec(), 0)
    }else{
        (
            bytes[0..(bytes.len()-8)].to_vec(),
            u64::from_be_bytes(bytes[(bytes.len()-8)..].try_into().unwrap())
        )
    }


}
pub fn unchop_table_key(bytes: &[u8], new_checkpoint_id: u64) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len()+8);
    result.extend_from_slice(bytes);
    result.extend_from_slice(&u64::to_be_bytes(new_checkpoint_id));

    result


}
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
pub struct ScyllaCheckpointStore{
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
}

impl ScyllaCheckpointStore {
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
        session
            .query_unpaged(
                q,
                &[],
            )
            .await?;
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
        })




    }


    pub async fn go_sel_15(&self, key_ids: &[Vec<u8>], checkpoint_id: u64) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        if key_ids.len() != 15 {
            anyhow::bail!("go_sel_15 called with {} key ids when it must have exactly 15",key_ids.len());
        }
        /*
        
        
                WHERE tree_id=? AND primary_id=? AND secondary_id=? 
                AND node_level IN (?, ?) AND node_index IN  (?, ?) AND checkpoint_id <= ? PER PARTITION LIMIT 1
                
                 */

        



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
            checkpoint_id as i64,
        )).await?;
        let res = result.into_rows_result()?;

        let mut final_result = [const { None }; 15];

        for row in res.rows::<(Vec<u8>, Vec<u8>)>()? {
            match row {
                Ok(a) => {
                    let (node_uuid, value) = a;
                    //let regen = get_partial_node_key(&node_uuid)?;

                    for (i, v) in key_ids.iter().enumerate(){
                        if v.eq(&node_uuid) {
                            final_result[i] = Some(value.clone());

                        }
                    }
                },
                Err(e) => println!("derser: {:?}",e),
            }

        }

        Ok(final_result.to_vec())
    }
    pub async fn get_many_values(&self, keys: &[Vec<u8>], checkpoint_id: u64) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());

        let full_batches = keys.len()/15;
        //let remainder = keys.len()%15;

        for batch_id in 0..full_batches {
            results.extend_from_slice(&self.go_sel_15(&keys[(batch_id*15)..((batch_id+1)*15)], checkpoint_id).await?);

        }
        for key in keys[full_batches*15..].iter() {
            let v = self.get_node_value_at_checkpoint(key, checkpoint_id).await?;
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
    pub async fn insert_many_nodes(&self, nodes: &[KVQPair<Vec<u8>, Vec<u8>>]) -> anyhow::Result<()> {

        if nodes.len() == 0 {
            return Ok(())
        }else if nodes.len() == 1 {
            return self.insert_node(&nodes[0].key, nodes[0].value.to_owned()).await;
        }
        let remainder_nodes = nodes.len()%MAX_PREPARED_INSERT_BATCH_SIZE;
        let full_batches = nodes.len()/MAX_PREPARED_INSERT_BATCH_SIZE;


        let mut nodes_iter = nodes.iter();


        for _ in 0..full_batches {
            let mut row = Vec::with_capacity(MAX_PREPARED_INSERT_BATCH_SIZE);
            for (_, node) in (0..MAX_PREPARED_INSERT_BATCH_SIZE).zip(&mut nodes_iter) {
                let (nkey, c) = chop_table_key(&node.key);

                row.push((
                    nkey,
                    c as i64,
                    node.value.to_bytes()?
                ));
            }
            self.session.batch(&self.insert_prepared_batches[MAX_PREPARED_INSERT_BATCH_SIZE-1], row).await?;
        }

        if remainder_nodes != 0 {
            let mut row = Vec::with_capacity(MAX_PREPARED_INSERT_BATCH_SIZE);
            for node in nodes_iter{
                let (nkey, c) = chop_table_key(&node.key);

                row.push((
                   nkey,
                   c as i64,
                    node.value.to_bytes()?
                ));
            }
            self.session.batch(&self.insert_prepared_batches[remainder_nodes-1], row).await?;

        }


        //Vec::with_capacity(nodes.len());





        Ok(())
    }
    pub async fn insert_node(&self, key: &[u8], value: Vec<u8>) -> anyhow::Result<()> {
        let (nkey, c) = chop_table_key(&key);


        self.session.execute_unpaged(&self.insert_prepared, (
            nkey,
            c as i64,
            value
        )).await?;

        Ok(())
    }
    pub async fn dump_all_nodes_debug(&self) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        let v = self.session.execute_unpaged(&self.dump_all_nodes_prepared, ()).await?;

        let res = v.into_rows_result()?;
        

        let mut results = Vec::new();
        for i in res.rows::<(Vec<u8>,i64,Vec<u8>)>()? {
            let (node_uuid, checkpoint_id, value ) = i?;

            


            results.push(KVQPair {
                key: unchop_table_key(&node_uuid, checkpoint_id as u64),
                value: value,
            });
        }

        Ok(results)




    }
    pub async fn get_latest_node(&self, unchopped_key: &[u8]) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        let (nkey, _) = chop_table_key(unchopped_key);

        let v = self.session.execute_unpaged(&self.select_latest_prepared, (
            &nkey,
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(i64,Vec<u8>)>()?;
        match res {
            Some(r) =>  Ok(Some(KVQPair {
                key: unchop_table_key(&nkey, r.0 as u64),
                value: r.1,
            })),
            None =>  Ok(None),
        }
    }

    pub async fn get_node_chop(&self, unchopped_key: &[u8]) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        let (nkey, max_checkpoint) = chop_table_key(unchopped_key);

        let v = self.session.execute_unpaged(&self.select_at_or_before_checkpoint_prepared, (
            &nkey,
            max_checkpoint as i64,
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(i64,Vec<u8>)>()?;
        match res {
            Some(r) =>  Ok(Some(KVQPair {
                key: unchop_table_key(&nkey, r.0 as u64),
                value: r.1,
            })),
            None =>  Ok(None),
        }
    }

    pub async fn get_node_value_at_checkpoint(&self, key: &[u8], checkpoint_id: u64) -> anyhow::Result<Option<Vec<u8>>> {
        let v = self.session.execute_unpaged(&self.select_value_at_or_before_checkpoint_prepared, (
            key,

            checkpoint_id as i64
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(Vec<u8>,)>()?;

        match res {
            Some(r) =>  Ok(Some(r.0)),
            None =>  Ok(None),
        }
    }


    pub async fn get_node_at_checkpoint(&self, key: &[u8], checkpoint_id: u64) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        let v = self.session.execute_unpaged(&self.select_at_or_before_checkpoint_prepared, (
            key,

            checkpoint_id as i64
        )).await?;

        let res = v.into_rows_result()?.maybe_first_row::<(i64,Vec<u8>)>()?;
        match res {
            Some(r) =>  Ok(Some(KVQPair {
                key: unchop_table_key(key, r.0 as u64),
                value: r.1,
            })),
            None =>  Ok(None),
        }
    }
}


#[async_trait]
impl KVQBinaryStoreReaderAsync for ScyllaCheckpointStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        let r = self.get_node_chop(key).await?;
        match r {
            Some(x) => {
                if x.key.eq(key) {
                    Ok(Some(x.value))
                }else{
                    Ok(None)
                }
            },
            None => Ok(None),
        }
    }
    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {

        let r = self.get_node_chop(key).await?;
        let v: Option<Vec<u8>> = match r {
            Some(x) => {
                if x.key.eq(key) {
                    Some(x.value)
                }else{
                    None
                }
            },
            None => None,
        };

        match v {
            Some(x) => Ok(x),
            None => anyhow::bail!("not found"),
        }
    }
    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut results = Vec::with_capacity(keys.len());
        for k in keys.iter() {
            results.push(self.get_exact(k).await?);
        }
        Ok(results)
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        if fuzzy_bytes != 8 {
            anyhow::bail!("unsupported number of fuzzy bytes: {}, must be 8", fuzzy_bytes);
        }

        let r = self.get_node_chop(key).await?;
        Ok(match r {
            Some(x) => Some(x.value),
            None => None,
        })

    }
    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        todo!("not implemented");
    }
    async fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>>{
        if fuzzy_bytes != 8 {
            anyhow::bail!("unsupported number of fuzzy bytes: {}, must be 8", fuzzy_bytes);
        }
        let r = self.get_node_chop(key).await?;

        Ok(r)

    }

    async fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<Vec<u8>>>> {

        if keys.len() == 0 {
            return Ok(Vec::new());
        }

        if fuzzy_bytes != 8 {
            anyhow::bail!("unsupported number of fuzzy bytes: {}, must be 8", fuzzy_bytes);
        }


        let chopped = keys.iter().map(|x| chop_table_key(&x)).collect::<Vec<_>>();
        let first_checkpoint = chopped[0].1;

        let not_all_same_checkpoint = chopped.iter().find(|x| {
            x.1 != first_checkpoint
        }).is_some();
        if not_all_same_checkpoint {
            let mut results = Vec::with_capacity(keys.len());
            for c in chopped.iter() {
                results.push(self.get_node_value_at_checkpoint(&c.0, c.1).await?);
            }

            Ok(results)
        }else{
            let results = self.get_many_values(&chopped.into_iter().map(|x|x.0).collect::<Vec<_>>(), first_checkpoint).await?;

            Ok(results)

        }

    }
    async fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {

        if keys.len() == 0 {
            return Ok(Vec::new());
        }

        if fuzzy_bytes != 8 {
            anyhow::bail!("unsupported number of fuzzy bytes: {}, must be 8", fuzzy_bytes);
        }


        let chopped = keys.iter().map(|x| chop_table_key(&x)).collect::<Vec<_>>();
        //let first_checkpoint = chopped[0].1;

        /* 
        let not_all_same_checkpoint = chopped.iter().find(|x| {
            x.1 != first_checkpoint
        }).is_some();*/
        let mut results = Vec::with_capacity(keys.len());
        for c in chopped.iter() {
            results.push(self.get_node_at_checkpoint(&c.0, c.1).await?);
        }

        Ok(results)

    }
}

#[async_trait]
impl KVQBinaryStoreWriterImmutableAsync for ScyllaCheckpointStore {
    async fn imm_set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.insert_node(&key, value).await?;


        Ok(())
    }
    async fn imm_set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()>{
        self.insert_node(&key, value.to_owned()).await?;

        Ok(())
    }
    async fn imm_set_many_ref<'a>(
        &self,
        items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>],
    ) -> anyhow::Result<()>{
        let good = items.iter().map(|item|{
            KVQPair {
                key: item.key.to_vec(),
                value: item.value.to_vec(),
            }
        }).collect::<Vec<_>>();
        self.insert_many_nodes(&good).await?;
        


        Ok(())
    }
    async fn imm_set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()>{

        self.insert_many_nodes(&items).await?;

        Ok(())
    }
    async fn imm_set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()>{

        let good = keys.iter().zip(values.iter()).map(|(key, value)|{
            KVQPair {
                key: key.to_vec(),
                value: value.to_vec(),
            }
        }).collect::<Vec<_>>();
        self.insert_many_nodes(&good).await?;

        Ok(())
    }

    async fn imm_delete(&self, key: &Vec<u8>) -> anyhow::Result<bool>{

        todo!("not implemented");
    }
    async fn imm_delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>>{


        todo!("not implemented");
    }
}