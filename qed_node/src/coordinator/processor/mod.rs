use super::args::CoordinatorProcessorArgs;
use crate::common::verifier::get_cached_generic_verifier;
use crate::coordinator::state::processor::CoordinatorConfig;
use crate::coordinator::state::processor::CoordinatorProcessorContext;
use anyhow::{bail, Context};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueConsumerAsyncImm,
    history_queue::{CheckpointHistoryQueueEmitterAsyncImm, CheckpointHistoryQueueConsumerAsyncImm},
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::WorkerEventTransmitterAsyncImm,
};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_data::{
    config::store_config::QEDFelt, traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync,
};
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::node::coordinator::{
    QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
};
use qed_store::queue::new_redis_async_pool;
use qed_store::queue::ProofStoreFred;
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::queue::rsmq_queue::CEQueueNotification;
use qed_core::config::network_constants::COORDINATOR_TO_REALM_CHANNEL;
use qed_store::store::QEDStore;
use std::time::Duration;

use qed_store::store::journal::{Journal, JournalStore};
use std::sync::Arc;
use tokio::time::{sleep_until, Instant};
use tracing::{debug, error, info, warn};
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl};
use qed_store::queue::redis_queue::NotificationQueue;
use crate::common::clock::SlotTimer;
use crate::common::slot;
use crate::common::slot::{LocalClock, Slot};
type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = QEDFelt;

pub struct CoordinatorProcessNode<
    JL: Journal,
    SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueConsumerAsyncImm,
    HQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + NotificationQueue<CEQueueNotification>,
    WQ: WorkerEventTransmitterAsyncImm,
    PS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
    ER: WorkerEventReceiverAsyncImm,
    TS: QProvingTaskStore,
> {
    pub ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, TS>,
    pub journal_store: JL,
    pub edge_command_queue: Arc<HQ>,
    pub proof_store: PS,
    pub event_receiver: ER,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
}

impl<
        JL: Journal,
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + NotificationQueue<CEQueueNotification>,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
        ER: WorkerEventReceiverAsyncImm,
        TS: QProvingTaskStore,
    > CoordinatorProcessNode<JL, SR, DQ, HQ, WQ, PS, ER, TS>
{
    pub fn new(
        ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, TS>,
        journal_store: JL,
        edge_command_queue: Arc<HQ>,
        proof_store: PS,
        event_receiver: ER,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
        coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
        task_store: Arc<QProvingTaskStoreImpl>,
    ) -> Self {
        Self {
            ctx,
            journal_store,
            edge_command_queue,
            proof_store,
            event_receiver,
            proof_verifier,
            coordinator_worker_circuits,
            task_store,
        }
    }

    pub async fn wait_for_produce_block(&mut self) -> anyhow::Result<bool> {
        // Get current checkpoint to listen from
        let latest_l2_block_state = self.ctx.store.get_latest_l2_block_state().await?;
        let notify_message = self.edge_command_queue.consume_item(COORDINATOR_TO_REALM_CHANNEL).await?;

        let latest_l2_block_state = self.ctx.store.get_latest_l2_block_state().await?;

        let CEQueueNotification::StartProduceBlock { next_checkpoint } = notify_message;
        debug!("coordinator: wait_for_produce_block: next_checkpoint: {}, latest_l2_block_state.checkpoint_id: {}",
            next_checkpoint, latest_l2_block_state.checkpoint_id);

        match next_checkpoint.cmp(&latest_l2_block_state.checkpoint_id) {
            std::cmp::Ordering::Equal => {
                info!("✅ Building new block for checkpoint {}", next_checkpoint);
                // No need to delete from history queue, it's already processed
                return Ok(false);
            }
            std::cmp::Ordering::Less => {
                warn!(
                    "⚠️ Outdated checkpoint {}, current {}",
                    next_checkpoint, latest_l2_block_state.checkpoint_id
                );
                // No need to delete from history queue, it's already processed
                return Ok(false);
            }
            std::cmp::Ordering::Greater
                if next_checkpoint - latest_l2_block_state.checkpoint_id > 1 =>
            {
                warn!(
                    "🚧 Future checkpoint {} too far ahead of {}",
                    next_checkpoint, latest_l2_block_state.checkpoint_id
                );
                return Ok(false);
            }
            std::cmp::Ordering::Greater => {
                return Ok(true);
            }
        }
    }

    pub async fn wait_for_make_block(&mut self) -> bool {
        match self.wait_for_produce_block().await {
            Ok(true) => {
                info!("✅ Successfully wait for produce block");
                true
            }
            Ok(false) => {
                info!("⚠️ No pending tasks, waiting for next checkpoint");
                tokio::time::sleep(Duration::from_millis(slot::SLOT_SIZE/2)).await;
                false
            }
            Err(e) => {
                error!("❌ Error waiting for produce block: {:?}", e);
                tokio::time::sleep(Duration::from_millis(slot::SLOT_SIZE/2)).await;
                false
            }
        }
    }
}

impl
    CoordinatorProcessNode<
        JournalStore<QEDStore>,
        JournalStore<QEDStore>,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        QProvingTaskStoreImpl,
    >
{
    pub async fn new_with_config(cp_config: CoordinatorProcessorArgs) -> anyhow::Result<Self> {
        let bb8_pool =
            new_redis_async_pool(&cp_config.redis_uri, cp_config.redis_pool_size as usize).await?;
        info!("🐶 redis pool initialized");
        let task_store = Arc::new(QProvingTaskStoreImpl::new(&cp_config.redis_uri, cp_config.redis_pool_size as usize)
            .await?);
        let q = ProofStoreRedisAsync::new(
            bb8_pool,
            cp_config.queue_args.queue_biz_key.clone(),
        )
        .await?;

        let qed_store = QEDStore::from_backend(cp_config.backend.to_backend()).await?;
        let qed_store = JournalStore::new(qed_store);

        match qed_store.initialize_store().await {
            Ok(checkpoint_id) if checkpoint_id == 0 => {
                qed_store.commit(0)?;
            }
            Ok(_) => {}
            Err(_) => {
                qed_store.rollback(0)?;
            }
        }

        // Use the same ProofStoreRedisAsync instance for history queue
        let edge_command_queue = Arc::new(q.clone());

        let coord_config = CoordinatorConfig::get_standard(0);

        let qps = Arc::new(q.clone());

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_processor_ctx = CoordinatorProcessorContext::new(
            coord_config,
            Arc::new(qed_store.clone()),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            task_store.clone(),
            Arc::clone(&proof_verifier),
        )
        .await?;

        use qed_core::config::network_constants::get_default_worker_public_key;
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library, get_default_worker_public_key::<F>());

        Ok(CoordinatorProcessNode::new(
            coordinator_processor_ctx,
            qed_store,
            edge_command_queue,
            q.clone(),
            q,
            proof_verifier,
            coordinator_worker_circuits,
            task_store,
        ))
    }

    pub async fn build_block_inner(&mut self, next_checkpoint_id: u64, slot: u64) -> anyhow::Result<()> {
        info!("Building block for checkpoint {}", next_checkpoint_id);
        let now = Instant::now();

        // Build block (task graph is handled inside ctx.build_block)
        self.ctx.build_block(slot).await?;
        info!("✅ Built block {} in {}ms", next_checkpoint_id, now.elapsed().as_millis());

        // Wait for all proving jobs to complete
        let prove_start = Instant::now();
        info!("🐶 Waiting for block proving jobs");
        self.ctx
            .prover_queue
            .wait_for_block_proving_jobs_imm(next_checkpoint_id)
            .await?;
        info!("✅ Proved block {} in {}ms", next_checkpoint_id, prove_start.elapsed().as_millis());

        Ok(())
    }

    pub async fn build_block(&mut self, slot: u64) -> anyhow::Result<u64> {
        let latest_l2_block_state = self.ctx.store.get_latest_l2_block_state().await?;
        let next_checkpoint_id = latest_l2_block_state.checkpoint_id + 1;

        // Check if there are pending tasks for this checkpoint
        if !self.ctx.has_pending_tasks(next_checkpoint_id).await
            .map_err(|e| anyhow::anyhow!("Failed to check pending tasks: {:?}", e))? {
            bail!(
                "No pending tasks for checkpoint {}, slot {}",
                next_checkpoint_id, slot
            );
        }

        // Build and prove the block
        if let Err(e) = self.build_block_inner(next_checkpoint_id, slot).await {
            self.journal_store.rollback(next_checkpoint_id)?;
            bail!("Failed to build and prove block: {:?}", e);
        }

        // Commit the changes
        self.journal_store.commit(next_checkpoint_id)?;
        Ok(next_checkpoint_id)
    }
}

pub async fn run_processor(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    let mut coordinator_processor = CoordinatorProcessNode::new_with_config(args).await?;
    let slot_timer= SlotTimer::new(LocalClock);
    let slot_timer_other = slot_timer.clone();
    loop {
        tokio::select! {
            is = coordinator_processor.wait_for_make_block() => {
                if !is {
                    continue;
                }
            }
            slot = slot_timer.wait_for_next_slot() => {
                info!("✅ Successfully wait for next slot: {}", slot);
            }
        }

        let slot = slot_timer_other.get_current_slot();
        match coordinator_processor.build_block(slot).await {
            Ok(checkpoint_id) => {
                info!(
                    "✅ Successfully built and committed block {}, slot {}",
                    checkpoint_id, slot
                );
            }
            Err(e) => {
                error!("❌ Failed to build block: {:?}, slot: {}", e, slot);
            }
        }
    }
}
