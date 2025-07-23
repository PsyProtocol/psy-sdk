pub mod event_proc_memory;
pub mod simple_async_coord;
pub mod simple_async_realm;
pub mod worker_state;

use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::queue::{new_redis_async_pool, ProofStoreRedisAsync};
use tracing::{info, warn};
pub use worker_state::*;

use std::{sync::Arc, time::Duration};

use plonky2::plonk::config::GenericConfig;
use qed_core::job::traits::QProofStoreAsyncImm;
use qed_crypto::common::{
    circuit_library::CircuitInfoLibrary, worker::QNextGenWorkerGenericProverAsyncMut,
};

use crate::{
    common::{jobs::JobReceiver, verifier::get_cached_generic_verifier},
    coordinator::edge::jobs::CoordinatorJobReceiver,
};

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;

pub async fn run_coordinator_scheduler_worker(
    redis_url: String,
    pool_size: usize,
    worker_queue_suffix: &str,
    notifications_queue_suffix: &str,
    proof_store_key_suffix: &str,
    proof_store_counters_suffix: &str,
    edge_url: String,
) -> anyhow::Result<()> {
    let job_receiver = CoordinatorJobReceiver::new(edge_url).await?;
    run_scheduler_worker(
        redis_url,
        pool_size,
        worker_queue_suffix,
        notifications_queue_suffix,
        proof_store_key_suffix,
        proof_store_counters_suffix,
        job_receiver,
    )
    .await
}

pub async fn run_scheduler_worker<JR: JobReceiver>(
    redis_url: String,
    pool_size: usize,
    worker_queue_suffix: &str,
    notifications_queue_suffix: &str,
    proof_store_key_suffix: &str,
    proof_store_counters_suffix: &str,
    job_receiver: JR,
) -> anyhow::Result<()> {
    let pool = new_redis_async_pool(redis_url.as_str(), pool_size).await?;
    // Create storage and queues
    let store = ProofStoreRedisAsync::new2(
        pool,
        &worker_queue_suffix,
        &notifications_queue_suffix,
        &proof_store_key_suffix,
        &proof_store_counters_suffix,
    )
    .await?;
    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    let coordinator_worker_circuits =
        QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);
    run_scheduler_worker_inner(
        &store,
        &job_receiver,
        &coordinator_worker_circuits,
        &proof_verifier.library,
    )
    .await?;
    Ok(())
}

async fn run_scheduler_worker_inner<
    PS: QProofStoreAsyncImm + Send + Sync,
    JR: JobReceiver,
    L: CircuitInfoLibrary<C, D> + Send + Sync,
    G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
    C: GenericConfig<D> + 'static,
    const D: usize,
>(
    store: &PS,
    job_receiver: &JR,
    prover: &G,
    library: &L,
) -> anyhow::Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let job_id = match job_receiver.get_next_ready_job().await {
            Ok(job_id) => job_id,
            Err(e) => {
                warn!("Error getting next ready job: {:?}", e);
                continue;
            }
        };
        match prover.worker_prove_mut_async(&store, library, job_id).await {
            Ok(proof) => {
                info!("Proved job: job_id={:?}", job_id);
                let output_id = job_id.get_output_id();
                store.set_proof_by_id(output_id, &proof).await?;
                job_receiver.submit_job_proof(job_id, proof).await?;
            }
            Err(e) => {
                job_receiver
                    .submit_job_proof(job_id, "".to_string())
                    .await?;
                if e.to_string().contains("unsupported circuit") {
                    warn!("Unsupported circuit");
                } else {
                warn!("Failed to prove job: err={:?}, job_id={:?}", e, job_id);
                }
            }
        };
    }
}
