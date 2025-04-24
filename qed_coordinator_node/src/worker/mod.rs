use anyhow::Ok;
use fred::prelude::ClientLike;
use fred::prelude::Config;
use fred::prelude::ReconnectPolicy;
use fred::types::Builder;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::traits::QProofStoreAsyncImm;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_crypto::common::circuit_library::CircuitInfoLibrary;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_crypto::common::worker::QNextGenWorkerGenericProverAsyncMut;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::worker::simple_async_coord::SimpleAsyncCoordinatorWorker;
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use std::{sync::Arc, time::Duration};

use crate::args::CoordinatorWorkerArgs;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct CoordinatorWorkerNode<
    PS: QProofStoreAsyncImm + Send + Sync,
    ER: WorkerEventReceiverAsyncImm,
    L: CircuitInfoLibrary<C, D> + Send + Sync,
    G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
> {
    pub store: PS,
    pub event_receiver: ER,
    pub prover: Arc<G>,
    pub library: L,
}

pub struct CoordinatorWorkerNodeConfig {
    pool_size: usize,
    redis_uri: String,
}

impl<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
    > CoordinatorWorkerNode<PS, ER, L, G>
{
    pub fn new(store: PS, event_receiver: ER, prover: Arc<G>, library: L) -> Self {
        Self {
            store,
            event_receiver,
            prover,
            library,
        }
    }
}

impl
    CoordinatorWorkerNode<
        ProofStoreFred,
        ProofStoreFred,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
    >
{
    pub async fn new_with_config(cw_config: CoordinatorWorkerNodeConfig) -> anyhow::Result<Self> {
        let config = Config::from_url(&cw_config.redis_uri)?;
        let pool = Builder::from_config(config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(10);
            })
            // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(cw_config.pool_size)?;

        pool.init().await?;

        let q = ProofStoreFred::new(pool.clone(), "wq1".to_string(), "nq1".to_string());

        let proof_verifier = get_cached_generic_verifier::<C, D>();

        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

        Ok(CoordinatorWorkerNode::new(
            q.clone(),
            q,
            coordinator_worker_circuits.into(),
            proof_verifier.library,
        ))
    }
}

pub async fn run_worker(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    let coordinator_worker = CoordinatorWorkerNode::new_with_config(CoordinatorWorkerNodeConfig {
        pool_size: args.coordinator_pool_size as usize,
        redis_uri: args.coordinator_redis_uri,
    })
    .await?;

    tracing::info!("Coordinator worker started");
    SimpleAsyncCoordinatorWorker::run_worker::<
        _,
        _,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
        C,
        D,
    >(
        &coordinator_worker.store,
        &coordinator_worker.event_receiver,
        &coordinator_worker.prover,
        &coordinator_worker.library,
    )
    .await?;
    Ok(())
}
