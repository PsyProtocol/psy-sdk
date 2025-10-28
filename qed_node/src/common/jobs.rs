use std::sync::Arc;

use crate::common::QEDProof;

pub const MESSAGE_CLAIM_JOB: &str = "CLAIM_JOB";
use alloy_primitives::{keccak256, B256};
use async_trait::async_trait;
use jsonrpsee::{
    core::RpcResult,
    http_client::{HttpClient, HttpClientBuilder},
    proc_macros::rpc,
};
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use psy_core::job::{
    id::{ProvingJobDataType, QProvingJobDataID},
    traits::QProofStoreReaderAsync,
};
use qed_prover::wallet::{
    secp_sign::{Eip712Signable, SignedRequest},
    secp_wallet::Wallet,
};
use psy_store::queue::task_queue::QJob;
use tracing::{debug, info, trace, warn};

#[rpc(server, client, namespace = "qed")]
pub trait JobSchedulerRpc {
    #[method(name = "get_pending_job")]
    async fn get_pending_job(&self, signed: SignedRequest<psy_data::config::store_config::QEDHash>) -> RpcResult<Option<QJob>>;

    #[method(name = "get_proof_by_id")]
    async fn get_proof_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>>;

    #[method(name = "get_bytes_by_id")]
    async fn get_bytes_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>>;

    #[method(name = "set_proof_by_id")]
    async fn set_proof_by_id(
        &self,
        job: QJob,
        proof: QEDProof,
        signed: SignedRequest<psy_data::config::store_config::QEDHash>,
    ) -> RpcResult<()>;
}

#[derive(Clone)]
pub struct JobClient {
    pub rpc_client: HttpClient,
}

impl JobClient {
    pub async fn new(rpc_url: String) -> anyhow::Result<Self> {
        let rpc_client = HttpClientBuilder::default().build(&rpc_url)?;
        Ok(Self { rpc_client })
    }
}

#[async_trait]
impl JobReceiver for JobClient {
    async fn get_next_job(&self, wallet: Arc<Wallet>, worker_public_key: &str) -> anyhow::Result<QJob> {
        loop {
            let mut signed_request = qed_prover::wallet::secp_sign::SignedRequest::sign_hashable(&wallet, &MESSAGE_CLAIM_JOB)?;
            signed_request.worker_public_key = worker_public_key.to_string();
            if let Some(job) = JobSchedulerRpcClient::get_pending_job(&self.rpc_client, signed_request).await? {
                return Ok(job);
            }
            trace!("No pending job found, sleeping for 500 millis");
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    async fn submit_job_proof(&self, job: QJob, proof: QEDProof, wallet: Arc<Wallet>, worker_public_key: &str) -> anyhow::Result<()> {
        trace!("Submitted job proof for job_id: {:?}", job);
        let mut signed = qed_prover::wallet::secp_sign::SignedRequest::sign_hashable(&wallet, &proof)?;
        signed.worker_public_key = worker_public_key.to_string();
        JobSchedulerRpcClient::set_proof_by_id(&self.rpc_client, job, proof, signed).await?;
        Ok(())
    }
}

#[async_trait]
impl QProofStoreReaderAsync for JobClient {
    async fn contains_id(&self, _id: QProvingJobDataID) -> anyhow::Result<bool> {
        unimplemented!()
    }

    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(&self, id: QProvingJobDataID) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let proof_bytes = JobSchedulerRpcClient::get_proof_by_id(&self.rpc_client, id).await?;
        let proof = bincode::deserialize(&proof_bytes).map_err(|e| anyhow::anyhow!(e))?;
        Ok(proof)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let bytes = JobSchedulerRpcClient::get_bytes_by_id(&self.rpc_client, id).await?;
        Ok(bytes)
    }

    async fn get_public_input_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<Vec<C::F>> {
        let proof_bytes = JobSchedulerRpcClient::get_proof_by_id(&self.rpc_client, id).await?;
        let proof: ProofWithPublicInputs<C::F, C, D> = bincode::deserialize(&proof_bytes).map_err(|e| anyhow::anyhow!(e))?;
        Ok(proof.public_inputs)
    }
}

#[async_trait]
pub trait JobReceiver {
    async fn get_next_job(&self, wallet: Arc<Wallet>, worker_public_key: &str) -> anyhow::Result<QJob>;

    async fn submit_job_proof(&self, job: QJob, proof: QEDProof, wallet: Arc<Wallet>, worker_public_key: &str) -> anyhow::Result<()>;
}
