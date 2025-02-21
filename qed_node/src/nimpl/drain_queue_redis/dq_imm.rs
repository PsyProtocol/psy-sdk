use kvq::traits::KVQSerializable;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::job::drain_queue::{CheckpointDrainQueueConsumerSyncImm, CheckpointDrainQueueEmitterSyncImm, DQSerializable};
use qed_core::job::id::QProvingJobDataID;
use qed_core::job::traits::{QProofStoreReaderSync, QProofStoreWriterSync, QProofStoreWriterSyncImm};
use r2d2_redis::RedisConnectionManager;
use redis::Commands;

// Table
pub const USER_STATE: &'static str = "user_state";

pub const PROOFS: &'static str = "proofs";
pub const PROOF_COUNTERS: &'static str = "proof_counters";

#[derive(Clone)]
pub struct DrainQueueRedis {
    pool: r2d2::Pool<RedisConnectionManager>,
}

impl DrainQueueRedis {
    pub fn new(uri: &str) -> anyhow::Result<Self> {
        let manager = RedisConnectionManager::new(uri)?;
        let pool = r2d2::Pool::builder().build(manager)?;
        Ok(Self { pool })
    }

    pub fn get_connection(&self) -> anyhow::Result<r2d2::PooledConnection<RedisConnectionManager>> {
        Ok(self.pool.get()?)
    }

    pub fn get_pool(&self) -> r2d2::Pool<RedisConnectionManager> {
        self.pool.clone()
    }/*

    pub fn get_user_state(&self, user_id: u64) -> anyhow::Result<CityUserState> {
        let mut connection = self.get_connection()?;
        let data: Vec<u8> = connection.hget(USER_STATE, user_id)?;
        Ok(bincode::deserialize(&data)?)
    }

    pub fn set_user_state(&self, user_state: &CityUserState) -> anyhow::Result<()> {
        let mut connection = self.get_connection()?;
        connection.hset(
            USER_STATE,
            user_state.user_id,
            bincode::serialize(user_state)?,
        )?;
        Ok(())
    }*/
}
/*
impl CheckpointDrainQueueEmitterSyncImm for DrainQueueRedis {
    fn cdq_push_imm_sync<T:DQSerializable>(&self,item:T) -> anyhow::Result<()>  {
        let metadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        let mut conn = self.get_connection()?;
        conn.sadd::<String, Vec<u8>, ()>(format!("CDQ_1_{}_{}",metadata.channel_id, metadata.checkpoint_id), bytes)?;

        Ok(())
    }
}

impl CheckpointDrainQueueConsumerSyncImm for DrainQueueRedis {
    fn cdq_drain_imm_sync<T:DQSerializable>(&self,channel_id:u64,checkpoint_id:u64,) -> anyhow::Result<Vec<T> >  {
        let mut conn = self.get_connection()?;
        let key = format!("CDQ_1_{}_{}",channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = conn.smembers::<String, Vec<Vec<u8>>>(key.clone())?;
        conn.del::<_, ()>(key)?;
        
        members.into_iter().map(|x| T::from_bytes(&x)).collect()
    }
}
*/
impl CheckpointDrainQueueEmitterSyncImm for DrainQueueRedis {
    fn cdq_push_imm_sync<T:DQSerializable>(&self,item:T) -> anyhow::Result<()>  {
        let metadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        let mut conn = self.get_connection()?;
        conn.lpush::<String, Vec<u8>, ()>(format!("CDQ_1_{}_{}",metadata.channel_id, metadata.checkpoint_id), bytes)?;

        Ok(())
    }
}

impl CheckpointDrainQueueConsumerSyncImm for DrainQueueRedis {
    fn cdq_drain_imm_sync<T:DQSerializable>(&self,channel_id:u64,checkpoint_id:u64,) -> anyhow::Result<Vec<T> >  {
        let mut conn = self.get_connection()?;
        let key = format!("CDQ_1_{}_{}",channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = conn.lrange::<String, Vec<Vec<u8>>>(key.clone(), 0, -1)?;
        conn.del::<_, ()>(key)?;
        
        members.into_iter().map(|x| T::from_bytes(&x)).collect()
    }
}
impl QProofStoreReaderSync for DrainQueueRedis {
    fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut conn = self.get_connection()?;
        let data: Vec<u8> = conn.hget(PROOFS, <[u8; 24]>::from(&id).to_vec())?;
        Ok(bincode::deserialize(&data)?)
    }

    fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let mut conn = self.get_connection()?;
        let data: Vec<u8> = conn.hget(PROOFS, <[u8; 24]>::from(&id).to_vec())?;
        Ok(data)
    }
}

impl QProofStoreWriterSync for DrainQueueRedis {
    fn set_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &mut self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        let mut conn = self.get_connection()?;
        conn.hset_nx::<_,_,_,()>(
            PROOFS,
            <[u8; 24]>::from(&id).to_vec(),
            bincode::serialize(&proof)?,
        )?;
        Ok(())
    }

    fn inc_counter_by_id(&mut self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let mut conn = self.get_connection()?;
        let value: u32 = conn.hincr(PROOF_COUNTERS, <[u8; 24]>::from(&id).to_vec(), 1)?;
        Ok(value)
    }

    fn set_bytes_by_id(&mut self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()> {
        let mut conn = self.get_connection()?;
        conn.hset_nx::<_,_,_,()>(PROOFS, <[u8; 24]>::from(&id).to_vec(), data)?;
        Ok(())
    }
    
    fn write_next_jobs(
        &mut self,
        jobs: &[QProvingJobDataID],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_next_jobs_core(jobs, next_jobs)
    }
    
    fn write_multidimensional_jobs(
        &mut self,
        jobs_levels: &[Vec<QProvingJobDataID>],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_multidimensional_jobs_core(jobs_levels, next_jobs)
    }
}


impl QProofStoreWriterSyncImm for DrainQueueRedis {
    fn set_proof_by_id_imm<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        let mut conn = self.get_connection()?;
        conn.hset_nx::<_,_,_,()>(
            PROOFS,
            <[u8; 24]>::from(&id).to_vec(),
            bincode::serialize(&proof)?,
        )?;
        Ok(())
    }

    fn inc_counter_by_id_imm(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let mut conn = self.get_connection()?;
        let value: u32 = conn.hincr(PROOF_COUNTERS, <[u8; 24]>::from(&id).to_vec(), 1)?;
        Ok(value)
    }

    fn set_bytes_by_id_imm(&self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()> {
        let mut conn = self.get_connection()?;
        conn.hset_nx::<_,_,_,()>(PROOFS, <[u8; 24]>::from(&id).to_vec(), data)?;
        Ok(())
    }
    
    fn write_next_jobs_imm(
        &self,
        jobs: &[QProvingJobDataID],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_next_jobs_core_imm(jobs, next_jobs)
    }
    
    fn write_multidimensional_jobs_imm(
        &self,
        jobs_levels: &[Vec<QProvingJobDataID>],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_multidimensional_jobs_core_imm(jobs_levels, next_jobs)
    }
}
