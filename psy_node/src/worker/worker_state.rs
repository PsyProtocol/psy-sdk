use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use psy_crypto::common::{generic_circuit_verifier::GenericCircuitVerifier, simple_circuit_library::SimpleCircuitLibrary};
use psy_data::config::store_config::PsyHasher;
use psy_network_circuit::coordinator::coordinator_helper::PsyCoordinatorCircuitManager;
use psy_store::queue::{new_redis_async_pool, ProofStoreRedis};

use crate::{
    common::verifier::get_cached_generic_verifier,
    worker::{simple_async_coord::SimpleAsyncCoordinatorWorker, simple_async_realm::SimpleAsyncRealmWorker},
};

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = psy_data::config::store_config::PsyFelt;

pub type H = PsyHasher;

#[derive(Debug)]
pub struct WorkerState {
    pub queue: ProofStoreRedis,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: PsyCoordinatorCircuitManager<C, D>,
}

impl WorkerState {
    pub async fn new(redis_url: String, pool_size: usize, biz_key: String) -> anyhow::Result<Self> {
        use psy_config::get_default_worker_public_key;
        // Create storage and queues
        let realm_qps = ProofStoreRedis::new(redis_url.as_str(), biz_key).await?;

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_worker_circuits =
            PsyCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library, get_default_worker_public_key::<F>());
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
        SimpleAsyncRealmWorker::run_worker::<_, _, SimpleCircuitLibrary<F>, PsyCoordinatorCircuitManager<C, D>, C, D>(
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
        SimpleAsyncCoordinatorWorker::run_worker::<_, _, SimpleCircuitLibrary<F>, PsyCoordinatorCircuitManager<C, D>, C, D>(
            &self.queue,
            &self.queue,
            &self.coordinator_worker_circuits,
            &self.proof_verifier.library,
        )
        .await
    }
}
