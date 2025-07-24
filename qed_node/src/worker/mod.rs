pub mod event_proc_memory;
pub mod simple_async_coord;
pub mod simple_async_realm;
pub mod worker_state;

use qed_core::job::{
    id::{QJobTopic, QProvingJobDataID},
    traits::QProofStoreReaderAsync,
};
use qed_crypto::common::worker::QNextGenWorkerGenericProverAsyncMut;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use std::{sync::Arc, time::Duration};
use tracing::{info, warn};
pub use worker_state::*;

use crate::{
    common::{jobs::JobReceiver, verifier::get_cached_generic_verifier},
    coordinator::edge::jobs::CoordinatorJobClient,
};

pub async fn run_coordinator_scheduler_worker(edge_url: String) -> anyhow::Result<()> {
    let job_client = CoordinatorJobClient::new(edge_url).await?;
    run_scheduler_worker(job_client.clone(), job_client).await
}

async fn run_scheduler_worker(
    job_receiver: impl JobReceiver,
    store: impl QProofStoreReaderAsync + Send + Sync,
) -> anyhow::Result<()> {
    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    let prover = QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);
    let library = &proof_verifier.library;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let job_id = match job_receiver.get_next_ready_job().await {
            Ok(job_id) => job_id,
            Err(e) => {
                warn!("Error getting next ready job: {:?}", e);
                continue;
            }
        };
        if !should_prove_job(job_id) {
            info!("skipping job proving: {:?}", job_id);
            job_receiver.submit_job_proof(job_id, None).await?;
            continue;
        }
        match prover.worker_prove_mut_async(&store, library, job_id).await {
            Ok(proof) => {
                info!("Proved job: job_id={:?}", job_id);
                job_receiver.submit_job_proof(job_id, Some(proof)).await?;
            }
            Err(e) => {
                warn!("Failed to prove job: err={:?}, job_id={:?}", e, job_id);
            }
        };
    }
}

fn should_prove_job(job_id: QProvingJobDataID) -> bool {
    job_id.topic == QJobTopic::GenerateStandardProof
}
