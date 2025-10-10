pub mod job_tracker;
pub mod simple_async_coord;
pub mod simple_async_realm;
pub mod worker_state;
pub mod client;
mod store_wrapper;

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
use std::{sync::Arc};
use std::time::Instant;
use tracing::{debug, error, info, warn, trace};
pub use worker_state::*;

use crate::common::{
    jobs::{JobClient, JobReceiver},
    verifier::get_cached_generic_verifier,
};
use job_tracker::{JobLocation, WorkerJobTracker};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout, Duration};
use qed_prover::wallet::secp_wallet::Wallet;
use crate::common::slot::SLOT_SIZE;
use qed_core::utils::trace_timer::TraceTimer;
use crate::worker::store_wrapper::RetryableStore;

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

    // Wrap the store with retry logic
    let base_store = job_client.clone();
    let store = Arc::new(RetryableStore::new(Arc::new(base_store)));

    let job_receiver = job_client;
    let worker_pk_str = worker_public_key.to_string();
    info!("⭐ worker public key = {}, user id = {}", worker_pk_str, user_id);
    loop {
        tokio::time::sleep(Duration::from_millis(200)).await;
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

        // Add a secondary timeout for proving retries
        const PROVING_RETRY_TIMEOUT_SECS: u64 = 30;
        let proving_start_time = Instant::now();
        let mut proving_attempt = 0;

        tokio::select! {
            _ = async {
                loop {
                    proving_attempt += 1;

                    // Check if we've exceeded the retry timeout
                    if proving_start_time.elapsed().as_secs() > PROVING_RETRY_TIMEOUT_SECS {
                        error!(
                            "Proving retry timeout exceeded ({} seconds) for job: {:?}, attempts: {}",
                            PROVING_RETRY_TIMEOUT_SECS, job_id, proving_attempt
                        );
                        break;
                    }

                    match prover.worker_prove_mut_async(store.as_ref(), library.as_ref(), job_id).await {
                        Ok(proof) => {
                            info!("Proved job: job_id={:?}, attempts={}", job_id, proving_attempt);

                            match job_receiver.submit_job_proof(job.clone(), proof, wallet.clone(), &worker_pk_str).await {
                                Ok(_) => {
                                    info!("Successfully submitted proof for job: {:?}, node: {:?}", job_id, location);
                                    {
                                        let mut tracker = job_tracker.lock().await;
                                        tracker.add_completed_job(job_id.get_output_id(), location.clone());
                                        if let Err(e) = tracker.save_to_file(&worker_pk_str) {
                                            error!("Failed to save job tracker: {:?}", e);
                                        }
                                    }
                                    timer.event(format!(
                                        "FINISHED job {} ({:?}) after {} attempts",
                                        job_id.to_hex_string(),
                                        job_id,
                                        proving_attempt
                                    ));
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to submit job proof: err={:?}, job_id={:?}, node: {:?}",
                                        e, job_id, location
                                    );
                                }
                            }
                            break;
                        }
                        Err(e) => {
                            let elapsed_secs = proving_start_time.elapsed().as_secs();

                            // Check if it's a deserialization error (likely from empty data)
                            if e.to_string().contains("IO error") || e.to_string().contains("deserialization") {
                                warn!(
                                    "Deserialization error for job {:?}, attempt {}, elapsed {}s: {:?}",
                                    job_id, proving_attempt, elapsed_secs, e
                                );

                                if elapsed_secs < PROVING_RETRY_TIMEOUT_SECS {
                                    // Sleep before retrying
                                    let backoff = Duration::from_millis(500 * proving_attempt.min(10) as u64);
                                    debug!("Sleeping for {:?} before retry", backoff);
                                    sleep(backoff).await;
                                    continue; // Retry
                                } else {
                                    error!(
                                        "Aborting job {:?} after {}s and {} attempts due to persistent errors",
                                        job_id, elapsed_secs, proving_attempt
                                    );
                                    break;
                                }
                            } else {
                                // Non-deserialization error, fail immediately
                                error!(
                                    "Failed to prove job: err={:?}, job_id={:?}, node: {:?}",
                                    e, job_id, location
                                );
                                break;
                            }
                        }
                    }
                }
            } => {}

            _ = tokio::time::sleep(timeout_duration) => {
                error!(
                    "Job proving timed out after {:?}: job_id={:?}, node: {:?}, attempts={}",
                    timeout_duration, job_id, location, proving_attempt
                );
            }
        }
    }
}
