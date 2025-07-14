use crate::args::CoordinatorProcessorArgs;
use crate::communicate::push_latest_global_coordinator_status;
use anyhow::bail;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::drain_queue::CheckpointDrainQueueEmitterAsyncImm;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueConsumerAsyncImm,
    history_queue::CheckpointHistoryQueueEmitterAsyncImm,
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::{ProvingDispatcher, ProvingWorkerListener, WorkerEventTransmitterAsyncImm},
};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_node::coordinator::state::processor::CoordinatorConfig;
use qed_node::coordinator::state::user_map::init_node_redis_pool;
use qed_node::nimpl::drain_queue_redis_async::dq_imm::DrainQueueRedisAsync;
use qed_node::nimpl::{new_redis_async_pool};
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
use std::{sync::Arc, time::Duration};
use tracing::{error, info, warn};
use kvq::cache::KVQBinaryStoreCached;
use qed_node::nimpl::proof_store_redis_async::ProofStoreRedisAsync;

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
    SQ: CheckpointDrainQueueEmitterAsyncImm + CheckpointDrainQueueConsumerAsyncImm,
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
        SQ: CheckpointDrainQueueEmitterAsyncImm + CheckpointDrainQueueConsumerAsyncImm,
    > CoordinatorProcessNode<SR, DQ, HQ, WQ, PS, ER, SQ>
{
    pub fn new(
        ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>,
        edge_command_queue: RedisQueue,
        sync_queue: Arc<SQ>,
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

    pub async fn wait_for_produce_block(
        &mut self,
        next_checkpoint_processor: u64,
    ) -> anyhow::Result<bool> {
        match self.edge_command_queue.pop_one(CE_NOTIFICATIONS)? {
            Some(message) => {
                let notify_message = serde_json::from_slice::<CEQueueNotification>(&message)?;

                match notify_message {
                    CEQueueNotification::StartProduceBlock { next_checkpoint } => {
                        let next_checkpoint_edge = next_checkpoint;
                        if next_checkpoint_edge == next_checkpoint_processor {
                            info!("✅ Building new block for checkpoint {}", next_checkpoint);
                            Ok(true)
                        } else if next_checkpoint_edge < next_checkpoint_processor {
                            warn!(
                                "⚠️ Outdated checkpoint {}, current {}",
                                next_checkpoint_edge, next_checkpoint_processor
                            );
                            Ok(false)
                        } else {
                            warn!(
                                "🚧 Future checkpoint {} too far ahead of {}",
                                next_checkpoint_edge, next_checkpoint_processor
                            );
                            self.edge_command_queue.dispatch(
                                CE_NOTIFICATIONS,
                                CEQueueNotification::StartProduceBlock { next_checkpoint },
                            )?;
                            Ok(false)
                        }
                    }
                }
            }
            None => Ok(false),
        }
    }
}

impl
    CoordinatorProcessNode<
        KVQArcImmutableStoreWrapper<KVQBinaryStoreCached<KVQlibmdbxStore>>,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        DrainQueueRedisAsync,
    >
{
    pub async fn new_with_config(cp_config: CoordinatorProcessorArgs) -> anyhow::Result<Self> {
        let pool = new_redis_async_pool(&cp_config.redis_uri, cp_config.pool_size as usize).await?;
        init_node_redis_pool(pool.clone())?;
        info!("🐶 redis pool initialized");
        let q = ProofStoreRedisAsync::new2(
            pool.clone(),
            &cp_config.queue_args.worker_queue_suffix,
            &cp_config.queue_args.notifications_queue_suffix,
            &cp_config.queue_args.proof_store_key_suffix,
            &cp_config.queue_args.proof_store_key_suffix,
        ).await?;
        let store_reader =
            KVQArcImmutableStoreWrapper::<KVQBinaryStoreCached<KVQlibmdbxStore>>::new(KVQBinaryStoreCached::new(KVQlibmdbxStore::new_write_with_size(
                &cp_config.db_path, cp_config.db_size_gb
            )?));

        //try to get the block 1's state
        let st = Arc::new(store_reader.dup());
        let need_init = match st.get_l2_block_state(1).await {
            Ok(_) => false,
            Err(e) => {
                error!(
                    "⚠️ Failed to get block 1 state: {:?}， need initialize the db",
                    e
                );
                if  QEDComboDataStoreReaderWriterSync::initialize_store(&*st)? == 0 {
                    st.commit_block(0).await?;
                }
                true
            }
        };

        //use sync_queue for checkpoint sync
        let sync_queue = Arc::new(DrainQueueRedisAsync::new(&cp_config.redis_uri).await?);
        let edge_command_queue = RedisQueue::new(&cp_config.redis_uri)?;

        let coord_config = CoordinatorConfig::get_standard(0);

        let qps = Arc::new(q.clone());

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

        //build block 1 only once
        if need_init {
            info!("build block 1");
            coordinator_processor_ctx.build_block().await?;
            push_latest_global_coordinator_status(sync_queue.clone(), 1, 1).await;
        }

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
    let mut coordinator_processor = CoordinatorProcessNode::new_with_config(args).await?;

    let latest_checkpoint_id = match coordinator_processor
        .ctx
        .store
        .get_latest_l2_block_state()
        .await
    {
        Ok(state) => state.checkpoint_id,
        Err(e) => {
            bail!("❌ Failed to get latest l2 block state: {:?}", e);
        }
    };

    let mut confirmed_checkpoint_id = latest_checkpoint_id;
    let mut next_checkpoint = latest_checkpoint_id + 1;
    info!(
        "🚀 Start coordinator processor at checkpoint {}",
        latest_checkpoint_id
    );

    let task = tokio::spawn(async move {
        let mut processor_loop = async move || -> anyhow::Result<()> {
            let mut last_logged_checkpoint = None;

            loop {
                if Some(next_checkpoint) != last_logged_checkpoint {
                    info!(
                        "wait for produce block {} command from coordinator edge",
                        next_checkpoint
                    );
                    last_logged_checkpoint = Some(next_checkpoint);
                }

                // wait for produce block message from coordinator edge
                let produce_ready = match coordinator_processor
                    .wait_for_produce_block(next_checkpoint)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        error!(
                            "❌ Error while waiting for produce block {}: {:?}",
                            next_checkpoint, e
                        );
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                };
                if !produce_ready {
                    tokio::time::sleep(Duration::from_millis(750)).await;
                    continue;
                }
                info!("start build block {}", next_checkpoint);
                if let Err(e) = coordinator_processor.ctx.build_block().await {
                    coordinator_processor.ctx.rollback_block(next_checkpoint).await?;
                    error!("❌ Failed to build block {}: {:?}", next_checkpoint, e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    bail!("❌ Failed to build block {}: {:?}", next_checkpoint, e);
                }
                coordinator_processor.ctx.commit_block(next_checkpoint).await?;
                info!("✅ Successfully built block {}", next_checkpoint);
                next_checkpoint += 1;
                push_latest_global_coordinator_status(
                    coordinator_processor.sync_queue.clone(),
                    confirmed_checkpoint_id,
                    next_checkpoint,
                )
                .await;
                let _job_id = match coordinator_processor
                    .ctx
                    .prover_queue
                    //indeed, we don't use this "next_checkpoint"
                    .wait_for_block_proving_jobs_imm(next_checkpoint)
                    .await
                {
                    Ok(job_id) => {
                        info!("✅ Proving job ready for block {}", &job_id.goal_id);
                        job_id
                    }
                    Err(e) => {
                        error!(
                            "❌ Failed to get proving job id for block {}: {:?}",
                            next_checkpoint - 1,
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                };
                confirmed_checkpoint_id += 1;

                push_latest_global_coordinator_status(
                    coordinator_processor.sync_queue.clone(),
                    confirmed_checkpoint_id,
                    next_checkpoint,
                )
                .await;
                tokio::time::sleep(Duration::from_millis(750)).await;
            }
        };

        processor_loop().await
    });

    match task.await {
        Ok(_) => info!("Coordinator processor task completed"),
        Err(e) => panic!("Coordinator processor task failed: {:?}", e),
    }

    Ok(())
}
