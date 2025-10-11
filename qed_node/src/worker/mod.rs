pub mod job_tracker;
pub mod simple_async_coord;
pub mod simple_async_realm;
pub mod worker_state;
pub mod client;

use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::{
    id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID},
    traits::QProofStoreReaderAsync,
};
use qed_crypto::common::{
    simple_circuit_library::SimpleCircuitLibrary, worker::QNextGenWorkerGenericProverAsyncMut,
};
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info, warn, trace};
pub use worker_state::*;

use crate::common::{
    jobs::{JobClient, JobReceiver},
    verifier::get_cached_generic_verifier,
    retry::{RetryConfig},
};
use job_tracker::{JobLocation, WorkerJobTracker};
use tokio::sync::Mutex;
use tokio::time::timeout;
use qed_prover::wallet::secp_wallet::Wallet;
use crate::common::slot::SLOT_SIZE;
use qed_core::utils::trace_timer::TraceTimer;
use crate::common::retry::retry_with_backoff;

pub async fn run_worker(
    edge_url: String,
    location: JobLocation,
    job_tracker: Arc<Mutex<WorkerJobTracker>>,
    prover: Arc<QEDCoordinatorCircuitManager<C, D>>,
    library: Arc<SimpleCircuitLibrary<F>>,
    wallet: Arc<Wallet>,
    worker_public_key: QHashOut<F>,
    user_id: u64,
) -> anyhow::Result<()> {
    info!("Running worker for edge: {}", edge_url);
    let job_client = JobClient::new(edge_url).await?;

    let store = job_client.clone();
    let job_receiver = job_client;
    let worker_pk_str = worker_public_key.to_string();
    info!("⭐ worker public key = {}, user id = {}", worker_pk_str, user_id);

    // Configure retry behavior based on location
    let retry_config = match location {
        JobLocation::Coordinator => RetryConfig {
            max_retries: 5,
            base_delay_ms: 1000,
            exponential_backoff: true,
        },
        JobLocation::Realm(_) => RetryConfig {
            max_retries: 3,
            base_delay_ms: 500,
            exponential_backoff: true,
        },
    };

    loop {
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get next job
        let job = match job_receiver.get_next_job(wallet.clone(), &worker_pk_str).await {
            Ok(job) => job,
            Err(e) => {
                warn!("Error getting next ready job: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let job_id = job.job_id;
        let mut timer = TraceTimer::new("process_job");
        timer.event(format!(
            "STARTED job {} ({:?})",
            job_id.to_hex_string(),
            job_id
        ));

        let timeout_duration = match location {
            JobLocation::Coordinator => Duration::from_millis(2 * SLOT_SIZE),
            JobLocation::Realm(_) => Duration::from_millis(SLOT_SIZE),
        };

        // Process job with timeout and retry logic
        match timeout(timeout_duration, process_job_with_retry(
            &store,
            &job_receiver,
            job.clone(),
            &prover,
            library.as_ref(),
            wallet.clone(),
            &worker_pk_str,
            &job_tracker,
            location.clone(),
            &retry_config,
            &mut timer,
        )).await {
            Ok(Ok(())) => {
                // Job completed successfully
                debug!("Job {} completed successfully", job_id.to_hex_string());
            }
            Ok(Err(e)) => {
                // Job failed after all retries
                error!(
                    "Job {} failed after all retries: {:?}, node: {:?}",
                    job_id.to_hex_string(), e, location
                );
            }
            Err(_) => {
                // Timeout occurred
                error!(
                    "Job proving timed out after {:?}: job_id={:?}, node: {:?}",
                    timeout_duration, job_id, location
                );
            }
        }
    }
}

async fn process_job_with_retry<S, R>(
    store: &S,
    job_receiver: &R,
    job: qed_store::queue::task_queue::QJob,
    prover: &Arc<QEDCoordinatorCircuitManager<C, D>>,
    library: &SimpleCircuitLibrary<F>,
    wallet: Arc<Wallet>,
    worker_pk_str: &str,
    job_tracker: &Arc<Mutex<WorkerJobTracker>>,
    location: JobLocation,
    retry_config: &RetryConfig,
    timer: &mut TraceTimer,
) -> anyhow::Result<()>
where
    S: QProofStoreReaderAsync + Send + Sync,
    R: JobReceiver + Send + Sync,
{
    let job_id = job.job_id;

    // Retry the proving operation
    let proof = retry_with_backoff(
        retry_config,
        &format!("prove job {}", job_id.to_hex_string()),
        || async {
            prover.worker_prove_mut_async(store, library, job_id).await
        },
    ).await?;

    info!("Proved job: job_id={:?}", job_id);

    // Submit proof with retry
    retry_with_backoff(
        retry_config,
        &format!("submit proof for job {}", job_id.to_hex_string()),
        || async {
            job_receiver.submit_job_proof(job.clone(), proof.clone(), wallet.clone(), worker_pk_str).await
        },
    ).await?;

    info!("Successfully submitted proof for job: {:?}, node: {:?}", job_id, location);

    // Update job tracker
    {
        let mut tracker = job_tracker.lock().await;
        tracker.add_completed_job(job_id.get_output_id(), location.clone());
        if let Err(e) = tracker.save_to_file(worker_pk_str) {
            error!("Failed to save job tracker: {:?}", e);
        }
    }

    timer.event(format!(
        "FINISHED job {} ({:?})",
        job_id.to_hex_string(),
        job_id
    ));

    Ok(())
}