
use fred::prelude::{ClientLike, Client};
use fred::prelude::Config;
use fred::prelude::ReconnectPolicy;
use fred::types::Builder;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueConsumerAsyncImm,
    history_queue::CheckpointHistoryQueueEmitterAsyncImm,
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::{ProvingDispatcher, ProvingWorkerListener, WorkerEventTransmitterAsyncImm},
};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_node::coordinator::state::processor::CoordinatorConfig;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::nimpl::worker_queue_redis::redis_queue::{CPQueueNotification};
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
use std::{ sync::Arc, time::Duration};
use anyhow::bail;
use tokio::time::sleep;
use tracing::{error, info, warn};
use qed_core::job::drain_queue::CheckpointDrainQueueEmitterAsyncImm;
use qed_node::coordinator::state::user_map::init_node_redis_pool;
use qed_node::nimpl::drain_queue_fred::DrainQueueFred;
use qed_node::nimpl::new_fred_pool;
use qed_realm_node::RedisConfig;
use crate::args::CoordinatorProcessorArgs;
use crate::{COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX, COORDINATOR_WORKER_QUEUE_SUFFIX, COORDINATOR_WORKER_SUFFIX};
use crate::communicate::push_latest_global_coordinator_status;
use crate::redis::{broadcast_checkpoint_sync};
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
    SQ: CheckpointDrainQueueConsumerAsyncImm + CheckpointDrainQueueEmitterAsyncImm,

> {
    pub ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>,
    pub sync_queue: Arc<SQ>,
    pub edge_command_queue: RedisQueue,
    pub proof_store: PS,
    pub event_receiver: ER,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
}

impl<
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
        ER: WorkerEventReceiverAsyncImm,
        SQ: CheckpointDrainQueueConsumerAsyncImm + CheckpointDrainQueueEmitterAsyncImm,

> CoordinatorProcessNode<SR, DQ, HQ, WQ, PS, ER, SQ>
{
    pub fn new(
        ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>,
        edge_command_queue: RedisQueue,
        sync_queue:  Arc<SQ>,
        proof_store: PS,
        event_receiver: ER,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
        coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
    ) -> Self {
        Self {
            ctx,
            edge_command_queue,
            sync_queue,
            proof_store,
            event_receiver,
            proof_verifier,
            coordinator_worker_circuits,
        }
    }

    pub async fn wait_for_produce_block(&mut self, next_checkpoint_processor: u64) -> anyhow::Result<bool> {
        match self.edge_command_queue.pop_one(CE_NOTIFICATIONS)? {
            Some(message) => {
                let notify_message = serde_json::from_slice::<CEQueueNotification>(&message)?;

                match notify_message {
                    CEQueueNotification::StartProduceBlock { next_checkpoint } => {
                        let next_checkpoint_edge = next_checkpoint;
                        if next_checkpoint_edge == next_checkpoint_processor  {
                            tracing::info!("✅ Building new block for checkpoint {}", next_checkpoint);
                            Ok(true)
                        } else if next_checkpoint_edge < next_checkpoint_processor {
                            tracing::warn!("⚠️ Outdated checkpoint {}, current {}", next_checkpoint_edge, next_checkpoint_processor);
                            Ok(false)
                        } else {
                            tracing::warn!("🚧 Future checkpoint {} too far ahead of {}", next_checkpoint_edge, next_checkpoint_processor);
                            self.edge_command_queue.dispatch(CE_NOTIFICATIONS, CEQueueNotification::StartProduceBlock { next_checkpoint })?;
                            Ok(false)
                        }
                    }
                    _ => Ok(false),
                }
            }
            None => Ok(false),
        }
    }
}

impl
    CoordinatorProcessNode<
        KVQArcImmutableStoreWrapper<KVQlibmdbxStore>,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
        DrainQueueFred,
    >
{
    pub async fn new_with_config(cp_config: CoordinatorProcessorArgs) -> anyhow::Result<Self> {
        let pool = new_fred_pool(&cp_config.coordinator_redis_uri, cp_config.coordinator_pool_size as usize).await?;
        init_node_redis_pool(pool.clone())?;
        info!("🐶 redis pool initialized");
        let q = ProofStoreFred::new2(
            pool.clone(),
            cp_config
                .coordinator_processor_queue_args
                .coordinator_worker_queue_suffix
                .clone(),
            cp_config
                .coordinator_processor_queue_args
                .coordinator_notifications_queue_suffix
                .clone(),
            Some(
                cp_config
                    .coordinator_processor_queue_args
                    .coordinator_proof_store_key_suffix
                    .as_str(),
            ),
            Some(
                cp_config
                    .coordinator_processor_queue_args
                    .coordinator_proof_store_key_suffix
                    .as_str(),
            ),
        );

        let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_write(
                &cp_config.coordinator_db_path,
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

        let sync_queue = Arc::new(DrainQueueFred::new(pool.clone()));
        let edge_command_queue = RedisQueue::new(&cp_config.coordinator_redis_uri)?;
        push_latest_global_coordinator_status(sync_queue.clone(), 1, 1).await?;

        //build block 1
        coordinator_processor_ctx.build_block().await?;

        // worker
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

        Ok(CoordinatorProcessNode::new(
            coordinator_processor_ctx,
            edge_command_queue,
            sync_queue,
            q.clone(),
            q,
            proof_verifier,
            coordinator_worker_circuits,
        ))
    }
}

pub async fn run_processor(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    // tracing_subscriber::fmt()
    //     .with_max_level(Level::DEBUG)
    //     .with_env_filter(EnvFilter::from_default_env())
    //     .init();
    //
    let mut coordinator_processor =
        CoordinatorProcessNode::new_with_config(args)
        .await?;

    let mut latest_checkpoint_id = match  coordinator_processor
        .ctx
        .store
        .get_latest_l2_block_state()
        .await {
        Ok(state) => state.checkpoint_id,
        Err(e) => {
            bail!("❌ Failed to get latest l2 block state: {:?}", e);
        }
    };
    let mut confirmed_checkpoint_id = latest_checkpoint_id;
    let mut next_checkpoint = latest_checkpoint_id + 1;
    tracing::info!("start coordinator processor");
    let task = tokio::spawn(async move {
        let mut processor_loop = async move || -> anyhow::Result<()> {
            loop {
                // wait for produceblock message from coordinator edge
                info!("wait for produce block {} command from coordinator edge", next_checkpoint);

                if coordinator_processor.wait_for_produce_block(next_checkpoint).await? {
                    tracing::info!("start build block {}", next_checkpoint);
                    coordinator_processor.ctx.build_block().await?;
                    next_checkpoint += 1;

                    // save latest coordinator status
                    push_latest_global_coordinator_status(coordinator_processor.sync_queue.clone(), confirmed_checkpoint_id, next_checkpoint).await?;

                    let _: qed_core::job::id::QProvingJobDataID = coordinator_processor
                        .ctx
                        .prover_queue
                        .wait_for_block_proving_jobs_imm(next_checkpoint)
                        .await?;
                    confirmed_checkpoint_id += 1;
                    // save latest coordinator status
                    push_latest_global_coordinator_status(coordinator_processor.sync_queue.clone(), confirmed_checkpoint_id, next_checkpoint).await?;

                    tracing::info!("save latest coordinator status {}", next_checkpoint);
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
