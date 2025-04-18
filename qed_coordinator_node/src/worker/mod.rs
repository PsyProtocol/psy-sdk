use anyhow::Ok;
use fred::prelude::ClientLike;
use fred::prelude::Config;
use fred::prelude::ReconnectPolicy;
use fred::types::Builder;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueConsumerAsyncImm,
    history_queue::CheckpointHistoryQueueEmitterAsyncImm,
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::{ProvingDispatcher, ProvingWorkerListener, WorkerEventTransmitterAsyncImm},
};
use qed_crypto::common::circuit_library::CircuitInfoLibrary;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_crypto::common::worker::QNextGenWorkerGenericProverAsyncMut;
use qed_node::coordinator::state::processor::CoordinatorConfig;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::nimpl::worker_queue_redis::redis_queue::{CPQueueNotification, CP_NOTIFICATIONS};
use qed_node::worker::simple_async_coord::SimpleAsyncCoordinatorWorker;
use qed_node::{
    coordinator::state::processor::CoordinatorProcessorContext,
    nimpl::worker_queue_redis::redis_queue::{CEQueueNotification, RedisQueue, CE_NOTIFICATIONS},
};
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::{
    config::store_config::QEDFelt,
    node::coordinator::store_traits::{
        QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
    },
    traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync,
};
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::subcommand::CoordinatorWorkerArgs;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = QEDFelt;

#[derive(Clone)]
pub struct CoordinatorWorkerNode<
    PS: QProofStoreAsyncImm + Send + Sync,
    ER: WorkerEventReceiverAsyncImm,
    L: CircuitInfoLibrary<C, D> + Send + Sync,
    G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
    // C: GenericConfig<D> + 'static,
    // const D: usize,
> {
    pub ctx: SimpleAsyncCoordinatorWorker,
    pub store: PS,
    pub event_receiver: ER,
    pub prover: Arc<G>,
    pub library: L,
}

pub struct CoordinatorWorkerNodeConfig {
    pool_size: usize,
    redis_url: String,
}

impl<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
    > CoordinatorWorkerNode<PS, ER, L, G>
{
    pub fn new(
        ctx: SimpleAsyncCoordinatorWorker,
        store: PS,
        event_receiver: ER,
        prover: Arc<G>,
        library: L,
    ) -> Self {
        Self {
            ctx,
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
        let config = Config::from_url(&cw_config.redis_url)?;
        let pool = Builder::from_config(config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(10);
            })
            // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(cw_config.pool_size)?;

        pool.init().await?;

        let q = ProofStoreFred::new(pool.clone(), "wq1".to_string(), "nq1".to_string());
        let flags = EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadWrite {
                sync_mode: SyncMode::Durable,
            },
            coalesce: true,
            ..Default::default()
        };

        let env = Environment::builder()
            .set_max_dbs(10)
            .set_flags(flags)
            .open(PathBuf::new().join("db").as_path())?;

        let txn = env.begin_rw_txn()?;
        let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RW>>::new(KVQlibmdbxStore::new(
                txn.clone(),
                None,
            )?);

        store_reader.initialize_store()?;

        let proof_verifier = get_cached_generic_verifier::<C, D>();

        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

        Ok(CoordinatorWorkerNode::new(
            SimpleAsyncCoordinatorWorker {},
            q.clone(),
            q,
            coordinator_worker_circuits.into(),
            proof_verifier.library,
        ))
    }
}

pub async fn run(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    let coordinator_worker = CoordinatorWorkerNode::new_with_config(CoordinatorWorkerNodeConfig {
        pool_size: args.pool_size as usize,
        redis_url: args.redis_url,
    })
    .await?;
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
