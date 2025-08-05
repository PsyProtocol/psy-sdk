use async_trait::async_trait;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_store::queue::{new_fred_pool, new_redis_async_pool};
use qed_store::queue::ProofStoreFred;
use crate::worker::simple_async_coord::SimpleAsyncCoordinatorWorker;
use crate::worker::simple_async_realm::SimpleAsyncRealmWorker;
use crate::common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_data::config::store_config::QEDHasher;
use std::ops::Deref;
use std::sync::Arc;
use qed_store::queue::ProofStoreRedisAsync;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_data::config::store_config::QEDFelt;

pub type H = QEDHasher;

#[derive(Debug)]
pub struct WorkerState {
    pub queue: ProofStoreRedisAsync,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
}

impl WorkerState {
    pub async fn new(
        redis_url: String,
        pool_size: usize,
        biz_key: String,
    ) -> anyhow::Result<Self> {
        let pool = new_redis_async_pool(
            redis_url.as_str(),
            pool_size
        ).await?;
        // Create storage and queues  
        let realm_qps = ProofStoreRedisAsync::new(
            pool,
            biz_key,
        ).await?;
        
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);
        let processor = WorkerState {
            queue: realm_qps,
            proof_verifier,
            coordinator_worker_circuits,
        };
        Ok(processor)
    }
}

pub struct RealmWorker(WorkerState);
pub struct CoordinatorWorker(WorkerState);

impl Deref for RealmWorker {
    type Target = WorkerState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for CoordinatorWorker {
    type Target = WorkerState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<WorkerState> for RealmWorker {
    fn from(worker: WorkerState) -> Self {
        RealmWorker(worker)
    }
}

impl From<WorkerState> for CoordinatorWorker {
    fn from(worker: WorkerState) -> Self {
        CoordinatorWorker(worker)
    }
}

#[async_trait]
pub trait Worker {
    async fn run(&self) -> anyhow::Result<()>;
}

#[async_trait]
impl Worker for RealmWorker {
    async fn run(&self) -> anyhow::Result<()> {
        SimpleAsyncRealmWorker::run_worker::<
            _,
            _,
            SimpleCircuitLibrary<F>,
            QEDCoordinatorCircuitManager<C, D>,
            C,
            D,
        >(
            &self.queue,
            &self.queue,
            &self.coordinator_worker_circuits,
            &self.proof_verifier.library,
        )
        .await
    }
}

#[async_trait]
impl Worker for CoordinatorWorker {
    async fn run(&self) -> anyhow::Result<()> {
        SimpleAsyncCoordinatorWorker::run_worker::<
            _,
            _,
            SimpleCircuitLibrary<F>,
            QEDCoordinatorCircuitManager<C, D>,
            C,
            D,
        >(
            &self.queue,
            &self.queue,
            &self.coordinator_worker_circuits,
            &self.proof_verifier.library,
        )
        .await
    }
}