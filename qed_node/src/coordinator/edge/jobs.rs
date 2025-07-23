use async_trait::async_trait;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use qed_core::job::id::QProvingJobDataID;
use serde::Serialize;
use tracing::info;

use crate::{common::jobs::JobReceiver, coordinator::edge::rpc::CoordinatorEdgeRpcClient};

pub struct CoordinatorJobReceiver {
    pub rpc_client: HttpClient,
}

impl CoordinatorJobReceiver {
    pub async fn new(rpc_url: String) -> anyhow::Result<Self> {
        info!("Creating coordinator job receiver: {}", rpc_url);
        let rpc_client = HttpClientBuilder::default().build(&rpc_url)?;
        Ok(Self { rpc_client })
    }
}

#[async_trait]
impl JobReceiver for CoordinatorJobReceiver {
    async fn get_next_ready_job(&self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            if let Some(job_id) =
                CoordinatorEdgeRpcClient::get_pending_job(&self.rpc_client).await?
            {
                return Ok(job_id);
            }
            info!("No pending job found, sleeping for 1 second");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
    async fn submit_job_proof<T: Serialize + Send + Sync>(
        &self,
        job_id: QProvingJobDataID,
        proof: T,
    ) -> anyhow::Result<()> {
        info!("Submitted job proof: {:?}", job_id);
        let proof_str = serde_json::to_string(&proof)?;
        CoordinatorEdgeRpcClient::set_proof_by_id(&self.rpc_client, job_id, proof_str).await?;
        Ok(())
    }
}
