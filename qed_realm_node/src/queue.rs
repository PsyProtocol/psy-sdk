use async_trait::async_trait;
use fred::prelude::{FredResult, ListInterface};
use kvq::traits::KVQSerializable;
use qed_core::job::id::ProvingJobDataId;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;

const REDIS_PROOF_KEY: &str = "REALM_PROOF";

#[async_trait]

pub trait RealmInternalQueue {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()>;

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId>;
}

#[async_trait]
impl RealmInternalQueue for ProofStoreFred {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()> {
        self.pool()
            .rpush::<(), &str, Vec<u8>>(REDIS_PROOF_KEY, item.to_bytes()?)
            .await?;
        Ok(())
    }

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId> {
        let result: FredResult<(String, Vec<u8>)> = self.pool().blpop(REDIS_PROOF_KEY, 0.0).await;

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
}
