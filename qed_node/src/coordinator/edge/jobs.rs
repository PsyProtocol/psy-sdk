use crate::{
    common::{jobs::JobReceiver, ConcreteProofWithPublicInputs},
    coordinator::edge::rpc::CoordinatorEdgeRpcClient,
};
use async_trait::async_trait;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use qed_core::job::{id::QProvingJobDataID, traits::QProofStoreReaderAsync};
use tracing::info;

#[derive(Clone)]
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
    async fn submit_job_proof(
        &self,
        job_id: QProvingJobDataID,
        proof: Option<ConcreteProofWithPublicInputs>,
    ) -> anyhow::Result<()> {
        info!("Submitted job proof: {:?}", job_id);
        // let proof_str = proof.map(|p| serde_json::to_string(&p)).transpose()?;
        CoordinatorEdgeRpcClient::set_proof_by_id(&self.rpc_client, job_id, proof).await?;
        Ok(())
    }
}

#[async_trait]
impl QProofStoreReaderAsync for CoordinatorJobReceiver {
    async fn contains_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        unimplemented!()
    }

    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let proof_bytes = CoordinatorEdgeRpcClient::get_proof_by_id(&self.rpc_client, id).await?;
        let proof = bincode::deserialize(&proof_bytes).map_err(|e| anyhow::anyhow!(e))?;
        Ok(proof)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let bytes = CoordinatorEdgeRpcClient::get_bytes_by_id(&self.rpc_client, id).await?;
        Ok(bytes)
    }
}
