pub mod job_tracker;
pub mod simple_async_coord;
pub mod simple_async_realm;
pub mod worker_state;

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
use tracing::{debug, error, info, warn};
pub use worker_state::*;

use crate::common::{
    jobs::{JobClient, JobReceiver},
    verifier::get_cached_generic_verifier,
};
use job_tracker::{JobLocation, WorkerJobTracker};
use tokio::sync::Mutex;
use qed_prover::wallet::secp_wallet::Wallet;

pub async fn run_worker(
    edge_url: String,
    location: JobLocation,
    job_tracker: Arc<Mutex<WorkerJobTracker>>,
    prover: Arc<QEDCoordinatorCircuitManager<C, D>>,
    library: Arc<SimpleCircuitLibrary<F>>,
    wallet: Arc<Wallet>,
    worker_public_key: QHashOut<F>,
) -> anyhow::Result<()> {
    info!("Running worker for edge: {}", edge_url);
    let job_client = JobClient::new(edge_url).await?;

    let store = job_client.clone();
    let job_receiver = job_client;

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let job = match job_receiver.get_next_job(wallet.clone()).await {
            Ok(job) => job,
            Err(e) => {
                warn!("Error getting next ready job: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        debug!("Received job, layer: {}", job.layer_id);
        let job_id = job.job_id;
        if !job_id.is_provable() {
            info!("skipping job proving: {:?}", job_id);
            job_receiver.submit_job_proof(job, None, wallet.clone()).await?;
            continue;
        }
        match prover
            .worker_prove_mut_async(&store, library.as_ref(), job_id)
            .await
        {
            Ok(proof) => {
                info!("Proved job: job_id={:?}", job_id);
                match job_receiver.submit_job_proof(job, Some(proof), wallet.clone()).await {
                    Ok(_) => {
                        info!("Successfully submitted proof for job: {:?}", job_id);
                        {
                            let mut tracker = job_tracker.lock().await;
                            tracker.add_completed_job(job_id, location.clone());
                            if let Err(e) = tracker.save_to_file(&worker_public_key.to_string()) {
                                error!("Failed to save job tracker: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to submit job proof: err={:?}, job_id={:?}",
                            e, job_id
                        );
                    }
                }
            }
            Err(e) => {
                error!("Failed to prove job: err={:?}, job_id={:?}", e, job_id);
            }
        };
    }
}
