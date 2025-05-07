use async_trait::async_trait;
use fred::prelude::{FredResult, ListInterface};
use tracing::{debug, info};
use kvq::traits::KVQSerializable;
use qed_core::job::id::ProvingJobDataId;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use crate::rpc::CheckpointSyncInfo;

const REAML_PROOF_KEY: &str = "REALM_PROOF";
const REAML_CHECKPOINT_KEY: &str = "REALM_CHECKPOINT";

#[async_trait]

pub trait RealmInternalQueue {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()>;

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId>;

    async fn produce_checkpoint_async_info(
        &self,
        item: CheckpointSyncInfo,
    ) -> anyhow::Result<()>;

    async fn consume_checkpoint_async_info(&self) -> anyhow::Result<CheckpointSyncInfo>;
}

#[async_trait]
impl RealmInternalQueue for ProofStoreFred {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()> {
        self.pool()
            .rpush::<(), &str, Vec<u8>>(REAML_PROOF_KEY, item.to_bytes()?)
            .await?;
        Ok(())
    }

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId> {
        let result: FredResult<(String, Vec<u8>)> = self.pool().blpop(REAML_PROOF_KEY, 0.0).await;

        match result {
            Ok((_, bytes)) => match ProvingJobDataId::from_bytes(&bytes) {
                Ok(id) => Ok(id),
                Err(err) => Err(anyhow::anyhow!(
                    "Failed to parse ProvingJobDataId: {:?}",
                    err
                )),
            },
            Err(err) => Err(anyhow::anyhow!("Error getting job_id from Redis {:?}", err)),
        }
    }

    async fn produce_checkpoint_async_info(
        &self,
        item: CheckpointSyncInfo,
    ) -> anyhow::Result<()> {
        debug!("Producing checkpoint async info to Redis: checkpoint_id before: {}", item.compact.l2_block_state.checkpoint_id);
        self.pool()
            .lpush::<(), &str, Vec<u8>>(REAML_CHECKPOINT_KEY, item.to_bytes()?)
            .await?;
        debug!("Checkpoint async info produced to Redis: checkpoint_id after: {}", item.compact.l2_block_state.checkpoint_id);
        Ok(())
    }

    async fn consume_checkpoint_async_info(&self) -> anyhow::Result<CheckpointSyncInfo> {
        let result: FredResult<(String, Vec<u8>)> =
            self.pool().brpop(REAML_CHECKPOINT_KEY, 0.0).await;

        match result {
            Ok((_, bytes)) => match CheckpointSyncInfo::from_bytes(&bytes) {
                Ok(info) => Ok(info),
                Err(err) => Err(anyhow::anyhow!(
                    "Failed to parse CheckpointSyncInfo: {:?}",
                    err
                )),
            },
            Err(err) => Err(anyhow::anyhow!(
                "Error getting checkpoint info from Redis {:?}",
                err
            )),
        }
    }
}
