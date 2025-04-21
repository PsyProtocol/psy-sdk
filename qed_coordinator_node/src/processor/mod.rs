use anyhow::Ok;
use fred::prelude::ClientLike;
use fred::prelude::Config;
use fred::prelude::ReconnectPolicy;
use fred::types::Builder;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueConsumerAsyncImm,
    history_queue::CheckpointHistoryQueueEmitterAsyncImm,
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::{ProvingDispatcher, ProvingWorkerListener, WorkerEventTransmitterAsyncImm},
};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
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
use tracing::Level;
use tracing_subscriber::EnvFilter;

use crate::subcommand::CoordinatorProcessorArgs;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = QEDFelt;

pub struct CoordinatorProcessNode<
    SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueConsumerAsyncImm,
    HQ: CheckpointHistoryQueueEmitterAsyncImm,
    WQ: WorkerEventTransmitterAsyncImm,
    PS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
    ER: WorkerEventReceiverAsyncImm,
> {
    pub ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>,
    pub sync_queue: RedisQueue,
    pub proof_store: PS,
    pub event_receiver: ER,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
}

pub struct CoordinatorProcessNodeConfig {
    pool_size: usize,
    redis_url: String,
    storage_db_path: String,
}

impl<
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
        ER: WorkerEventReceiverAsyncImm,
    > CoordinatorProcessNode<SR, DQ, HQ, WQ, PS, ER>
{
    pub fn new(
        ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>,
        sync_queue: RedisQueue,
        proof_store: PS,
        event_receiver: ER,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
        coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
    ) -> Self {
        Self {
            ctx,
            sync_queue,
            proof_store,
            event_receiver,
            proof_verifier,
            coordinator_worker_circuits,
        }
    }

    pub async fn wait_for_produce_block(&mut self) -> anyhow::Result<bool> {
        match self.sync_queue.pop_one(CE_NOTIFICATIONS)? {
            Some(message) => {
                let notify_message = serde_json::from_slice::<CEQueueNotification>(&message)?;
                match notify_message {
                    CEQueueNotification::StartProduceBlock => Ok(true),
                    _ => Ok(false),
                }
            }
            None => Ok(false),
        }
    }

    pub async fn notify_sync(&mut self) -> anyhow::Result<()> {
        self.sync_queue
            .dispatch(CP_NOTIFICATIONS, CPQueueNotification::StartSync)?;
        Ok(())
    }
}

impl
    CoordinatorProcessNode<
        KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>>,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
    >
{
    pub async fn new_with_config(cp_config: CoordinatorProcessNodeConfig) -> anyhow::Result<Self> {
        let config = Config::from_url(&cp_config.redis_url)?;
        let pool = Builder::from_config(config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(10);
            })
            // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(cp_config.pool_size)?;

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
            .open(PathBuf::new().join(cp_config.storage_db_path).as_path())?;

        let txn = env.begin_rw_txn()?;
        let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RW>>::new(KVQlibmdbxStore::new(
                txn.clone(),
                None,
            )?);

        store_reader.initialize_store()?;

        let coord_config = CoordinatorConfig::get_standard(0);

        let qps = Arc::new(q.clone());

        let st = Arc::new(store_reader.dup());

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_processor_ctx = CoordinatorProcessorContext::new(
            coord_config,
            Arc::clone(&st),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            Arc::clone(&proof_verifier),
        )
        .await?;

        let sync_queue = RedisQueue::new(&cp_config.redis_url)?;

        // worker
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

        Ok(CoordinatorProcessNode::new(
            coordinator_processor_ctx,
            sync_queue,
            q.clone(),
            q,
            proof_verifier,
            coordinator_worker_circuits,
        ))
    }
}

pub async fn run(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mut coordinator_processor =
        CoordinatorProcessNode::new_with_config(CoordinatorProcessNodeConfig {
            pool_size: args.pool_size as usize,
            redis_url: args.redis_uri,
            storage_db_path: args.storage_db_path,
        })
        .await?;

    tracing::info!("start coordinator processor");
    let task = tokio::spawn(async move {
        let mut processor_loop = async move || -> anyhow::Result<()> {
            loop {
                // wait for produceblock message from coordinator edge
                tracing::info!("wait for produce_block message from coordinator edge");
                if coordinator_processor.wait_for_produce_block().await? {
                    tracing::info!("start build block");
                    coordinator_processor.ctx.build_block().await?;

                    tracing::info!("start worker");
                    SimpleAsyncCoordinatorWorker::run_worker_until_done::<
                        _,
                        _,
                        SimpleCircuitLibrary<GoldilocksField>,
                        QEDCoordinatorCircuitManager<C, D>,
                        C,
                        D,
                    >(
                        &coordinator_processor.proof_store,
                        &coordinator_processor.event_receiver,
                        &coordinator_processor.coordinator_worker_circuits,
                        &coordinator_processor.proof_verifier.library,
                    )
                    .await?;

                    tracing::info!("send sync message to coordinator edge");
                    // send sync message to coordinator edge
                    coordinator_processor.notify_sync().await?;
                }
                tokio::time::sleep(Duration::from_millis(750)).await;
            }
        };

        processor_loop().await
    });

    match task.await {
        std::result::Result::Ok(_) => tracing::info!("Coordinator processor task completed"),
        Err(e) => panic!("Coordinator processor task failed: {:?}", e),
    }

    Ok(())
}
