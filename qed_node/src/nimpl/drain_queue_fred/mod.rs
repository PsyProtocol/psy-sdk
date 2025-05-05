use fred::prelude::{KeysInterface, ListInterface, Pool};
use async_trait::async_trait;
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use qed_core::job::{drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable}, id::QProvingJobDataID, traits::QProofStoreReaderAsync};



#[derive(Clone)]
pub struct DrainQueueFred {
    pool: Pool,
}

impl DrainQueueFred {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}
/*

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
/*
impl CheckpointDrainQueueEmitterSyncImm for DrainQueueFred {
    fn cdq_push_imm_sync<T:DQSerializable>(&self,item:T) -> anyhow::Result<()>  {
        let metadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        let mut conn = self.get_connection()?;
        conn.sadd::<String, Vec<u8>, ()>(format!("CDQ_1_{}_{}",metadata.channel_id, metadata.checkpoint_id), bytes)?;

        Ok(())
    }
}

impl CheckpointDrainQueueConsumerSyncImm for DrainQueueFred {
    fn cdq_drain_imm_sync<T:DQSerializable>(&self,channel_id:u64,checkpoint_id:u64,) -> anyhow::Result<Vec<T> >  {
        let mut conn = self.get_connection()?;
        let key = format!("CDQ_1_{}_{}",channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = conn.smembers::<String, Vec<Vec<u8>>>(key.clone())?;
        conn.del::<_, ()>(key)?;

        members.into_iter().map(|x| T::from_bytes(&x)).collect()
    }
}
*/

#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for DrainQueueFred {
    async fn cdq_push_imm<T:DQSerializable>(&self,item:T) -> anyhow::Result<()>  {
        let metadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        self.pool.lpush::<(), String, &[u8]>(format!("CDQ_2_{}_{}",metadata.channel_id, metadata.checkpoint_id), &bytes).await?;

        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for DrainQueueFred {
    async fn cdq_get_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let key = format!("CDQ_2_{}_{}",channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = self.pool.lrange::<Vec<Vec<u8>>, String>(key.clone(), 0, -1).await?;
        members.into_iter().map(|x| T::from_bytes(&x)).collect()
    }

    async fn cdq_drain_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let key = format!("CDQ_2_{}_{}",channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = self.pool.lrange::<Vec<Vec<u8>>, String>(key.clone(), 0, -1).await?;
        self.pool.del::<(), String>(key).await?;

        members.into_iter().rev().map(|x| T::from_bytes(&x)).collect()
    }
}
