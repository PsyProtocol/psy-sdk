use std::{collections::HashMap, sync::Arc};

use crate::common::ConcreteProofWithPublicInputs;
use async_trait::async_trait;
use jsonrpsee::{
    core::RpcResult,
    http_client::{HttpClient, HttpClientBuilder},
    proc_macros::rpc,
};
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use qed_core::job::{
    id::{JobDataIdGraph, QProvingJobDataID},
    traits::{JobDataIdGraphReader, QProofStoreReaderAsync},
};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[rpc(server, client, namespace = "qed")]
pub trait JobSchedulerRpc {
    #[method(name = "get_pending_job")]
    async fn get_pending_job(&self) -> RpcResult<Option<QProvingJobDataID>>;

    #[method(name = "get_proof_by_id")]
    async fn get_proof_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>>;

    #[method(name = "get_bytes_by_id")]
    async fn get_bytes_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>>;

    #[method(name = "set_proof_by_id")]
    async fn set_proof_by_id(
        &self,
        job_id: QProvingJobDataID,
        proof: Option<ConcreteProofWithPublicInputs>,
    ) -> RpcResult<()>;
}

#[derive(Clone)]
pub struct JobClient {
    pub rpc_client: HttpClient,
}

impl JobClient {
    pub async fn new(rpc_url: String) -> anyhow::Result<Self> {
        info!("Creating coordinator job receiver: {}", rpc_url);
        let rpc_client = HttpClientBuilder::default().build(&rpc_url)?;
        Ok(Self { rpc_client })
    }
}

#[async_trait]
impl JobReceiver for JobClient {
    async fn get_next_ready_job(&self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            if let Some(job_id) = JobSchedulerRpcClient::get_pending_job(&self.rpc_client).await? {
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
        info!("Submitted job proof for job_id: {:?}", job_id);
        JobSchedulerRpcClient::set_proof_by_id(&self.rpc_client, job_id, proof).await?;
        Ok(())
    }
}

#[async_trait]
impl QProofStoreReaderAsync for JobClient {
    async fn contains_id(&self, _id: QProvingJobDataID) -> anyhow::Result<bool> {
        unimplemented!()
    }

    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let proof_bytes = JobSchedulerRpcClient::get_proof_by_id(&self.rpc_client, id).await?;
        let proof = bincode::deserialize(&proof_bytes).map_err(|e| anyhow::anyhow!(e))?;
        Ok(proof)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let bytes = JobSchedulerRpcClient::get_bytes_by_id(&self.rpc_client, id).await?;
        Ok(bytes)
    }
}

pub async fn run_jobs_listener<T: JobDataIdGraphReader + Send + Sync + 'static>(
    job_manager: Arc<Mutex<JobsGraphManager>>,
    job_graph_reader: Arc<T>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            match job_graph_reader.wait_for_next_job_graph().await {
                Ok((checkpoint_id, job_graph)) => {
                    println!("Received job graph, checkpoint_id = {}", checkpoint_id);
                    for job in job_graph.dep_graph.keys() {
                        println!("Job: {:?}", job);
                        for dep in job_graph.dep_graph.get(job).unwrap() {
                            println!("- Dep: {:?}", dep);
                        }
                    }
                    let mut job_manager = job_manager.lock().await;
                    if job_manager.is_job_graph_empty() {
                        job_manager.set_job_graph(job_graph, checkpoint_id);
                    } else {
                        error!(
                        "🔍 Job graph is not empty, skipping, checkpoint_id = {}, job_graph = {:?}",
                        checkpoint_id, job_graph
                    );
                    }
                }
                Err(err) => {
                    let err_message = err.to_string();
                    if err_message.contains("unexpected end of file") {
                        warn!("Job graph is empty, skipping");
                    } else {
                        warn!("Failed to read job graph: {:?}", err);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Processing,
    Done,
}

pub struct JobsGraphManager {
    pub job_graph: Option<JobDataIdGraph>,
    pub ready_jobs: HashMap<QProvingJobDataID, JobStatus>,
    pub checkpoint_id: u64,
}

impl JobsGraphManager {
    pub fn new() -> Self {
        Self {
            job_graph: None,
            ready_jobs: HashMap::new(),
            checkpoint_id: 0,
        }
    }

    pub fn set_job_graph(&mut self, job_graph: JobDataIdGraph, checkpoint_id: u64) {
        self.job_graph = Some(job_graph);
        self.checkpoint_id = checkpoint_id;
    }

    pub fn is_job_graph_empty(&self) -> bool {
        self.job_graph
            .as_ref()
            .map(|graph| graph.is_empty())
            .unwrap_or(true)
    }

    fn take_read_job(&mut self) {
        let ready_jobs = self.job_graph.as_mut().map(|graph| graph.take_ready_jobs());
        if let Some(ready_jobs) = ready_jobs.as_ref() {
            for &job_id in ready_jobs.iter() {
                self.ready_jobs.insert(job_id, JobStatus::Pending);
            }
        }
    }

    fn get_pending_job(&self) -> Option<QProvingJobDataID> {
        self.ready_jobs
            .iter()
            .find(|(_, &status)| status == JobStatus::Pending)
            .map(|(&job_id, _)| job_id)
    }

    fn are_ready_jobs_all_done(&self) -> bool {
        if self.ready_jobs.is_empty() {
            return true;
        }
        self.ready_jobs
            .values()
            .all(|&status| status == JobStatus::Done)
    }

    pub fn get_next_pending_job_to_process(&mut self) -> Option<QProvingJobDataID> {
        if self.are_ready_jobs_all_done() {
            self.take_read_job();
        }
        let pending_job = self.get_pending_job();
        if let Some(job_id) = pending_job {
            self.mark_job_status(job_id, JobStatus::Processing);
            info!("Found pending job: {:?}", job_id);
            Some(job_id)
        } else {
            if let Some(job_graph) = self.job_graph.as_ref() {
                if !job_graph.is_empty() {
                    error!("Job graph is not empty, but no pending job found");
                }
            }
            None
        }
    }

    pub fn mark_job_done(&mut self, job_id: QProvingJobDataID) {
        info!("Setting job done: {:?}", job_id);
        self.mark_job_status(job_id, JobStatus::Done);
    }

    pub fn mark_job_status(&mut self, job_id: QProvingJobDataID, new_status: JobStatus) {
        if let Some(status) = self.ready_jobs.get_mut(&job_id) {
            *status = new_status;
        } else {
            warn!("Job not found in ready_jobs: {:?}", job_id);
        }
    }
}

#[async_trait]
pub trait JobReceiver {
    async fn get_next_ready_job(&self) -> anyhow::Result<QProvingJobDataID>;

    async fn submit_job_proof(
        &self,
        job_id: QProvingJobDataID,
        proof: Option<ConcreteProofWithPublicInputs>,
    ) -> anyhow::Result<()>;
}
