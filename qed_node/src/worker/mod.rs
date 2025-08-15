pub mod simple_async_coord;
pub mod simple_async_realm;
pub mod worker_state;

use qed_core::data::qhashout::QHashOut;
use qed_core::job::{
    id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID},
    traits::QProofStoreReaderAsync,
};
use plonky2::field::goldilocks_field::GoldilocksField;
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

pub async fn run_worker(edge_urls: Vec<String>, worker_public_key: QHashOut<GoldilocksField>) -> anyhow::Result<()> {
    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    let prover = Arc::new(QEDCoordinatorCircuitManager::<C, D>::new_with_library(
        &proof_verifier.library,
        worker_public_key,
    ));
    let library = Arc::new(proof_verifier.library.clone());
    for edge_url in edge_urls {
        info!("Running worker for edge: {}", edge_url);
        let job_client = JobClient::new(edge_url).await?;
        let prover = prover.clone();
        let library = library.clone();
        tokio::spawn(async move {
            run_scheduler_worker(prover, library, job_client.clone(), job_client, worker_public_key)
                .await
                .expect("Failed to run scheduler worker");
        });
    }
    Ok(())
}

async fn run_scheduler_worker(
    prover: Arc<QEDCoordinatorCircuitManager<C, D>>,
    library: Arc<SimpleCircuitLibrary<F>>,
    job_receiver: impl JobReceiver,
    store: impl QProofStoreReaderAsync + Send + Sync,
    worker_public_key: QHashOut<GoldilocksField>,
) -> anyhow::Result<()> {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let job = match job_receiver.get_next_job().await {
            Ok(job) => job,
            Err(e) => {
                warn!("Error getting next ready job: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        debug!("Received job, layer: {}", job.task_id);
        let job_id = job.job_id;
        if !should_prove_job(job_id) {
            info!("skipping job proving: {:?}", job_id);
            job_receiver.submit_job_proof(job, None).await?;
            continue;
        }
        match prover
            .worker_prove_mut_async(&store, library.as_ref(), job_id)
            .await
        {
            Ok(proof) => {
                info!("Proved job: job_id={:?}", job_id);
                if let Err(e) = job_receiver.submit_job_proof(job, Some(proof)).await {
                    error!(
                        "Failed to submit job proof: err={:?}, job_id={:?}",
                        e, job_id
                    );
                }
            }
            Err(e) => {
                error!("Failed to prove job: err={:?}, job_id={:?}", e, job_id);
            }
        };
    }
}

fn should_prove_job(job_id: QProvingJobDataID) -> bool {
    job_id.topic == QJobTopic::GenerateStandardProof
        && job_id.circuit_type != ProvingJobCircuitType::NotifyRealmComplete
}
