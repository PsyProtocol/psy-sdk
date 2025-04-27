use async_trait::async_trait;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::worker::simple_async_coord::SimpleAsyncCoordinatorWorker;
use qed_node::worker::simple_async_realm::SimpleAsyncRealmWorker;
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::config::store_config::QEDHasher;
use std::ops::Deref;
use std::sync::Arc;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_store::config::store_config::QEDFelt;

pub type H = QEDHasher;

#[derive(Debug)]
pub struct WorkerState {
    pub queue: ProofStoreFred,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
}

impl WorkerState {
    pub async fn new(
        redis_url: String,
        pool_size: usize,
        worker_queue_suffix: String,
        notifications_queue_suffix: String,
        proof_store_key_suffix: Option<&str>,
        proof_store_counters_suffix: Option<&str>,
    ) -> anyhow::Result<Self> {
        let pool = new_fred_pool(&redis_url, pool_size).await?;
        let realm_qps = ProofStoreFred::new2(
            pool,
            worker_queue_suffix,
            notifications_queue_suffix,
            proof_store_counters_suffix,
            proof_store_key_suffix,
        );
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
