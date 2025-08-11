mod slot_phase;

use std::ops::Deref;
use crate::common::verifier::get_cached_generic_verifier;
use crate::realm::config::RealmNodeConfig;
use crate::realm::state::processor::{RealmConfig, RealmProcessorContext};
use crate::realm::{C, D, F};
use qed_core::job::history_queue::{
    CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
};
use qed_core::job::id::ProvingJobDataId;
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_store::queue::task_queue::{JobTaskStore, JobTaskStoreImpl};
use qed_store::store::QEDStore;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use anyhow::{anyhow, bail};
use futures::future::{err, ok};
use tower_http::follow_redirect::policy::PolicyExt;
use tracing::{debug, error, info, warn};
use qed_store::queue::new_redis_async_pool;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::store::journal::{Journal, JournalStore};
use crate::common::clock::SlotTimer;
use crate::common::slot::{Clock, LocalClock, Slot};
use crate::realm::processor::slot_phase::SlotPhase;

type ConcreteRealmProcessorContext = RealmProcessorContext<
    JournalStore<QEDStore>,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
>;

pub struct RealmProcessor {
    pub realm_config: RealmConfig,
    pub sync_proof: ProofStoreRedisAsync,
    pub sync_checkpoint: Arc<ProofStoreRedisAsync>,
    pub store: Arc<JournalStore<QEDStore>>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub job_task_store: Arc<JobTaskStoreImpl>,
    pub slot_timer: SlotTimer<LocalClock>,
    pub remote_latest_slot: u64,
}

pub async fn run_realm_processor(config: RealmNodeConfig) -> anyhow::Result<()> {
    let realm_processor = RealmProcessor::new(config).await?;
    let _ = realm_processor.start().await?;
    Ok(())
}

impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig) -> anyhow::Result<Self> {
        info!("Realm Processor Config: {:?}", config);
        let pool = new_redis_async_pool(
            config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10)
        ).await?;
        let task_store = JobTaskStoreImpl::new(
            &config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10),
        )
        .await?;
        let realm_qps = ProofStoreRedisAsync::new(
            pool,
            config.queue.queue_biz_key,
        ).await?;
        let store = QEDStore::new(&config.backend.to_backend()).await?;
        let store = Arc::new(JournalStore::new(store));
        let store_reader = store.clone();

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
        // Use the same ProofStoreRedisAsync for checkpoint sync
        let sync_checkpoint = Arc::new(realm_qps.clone());
        let processor = RealmProcessor {
            realm_config,
            sync_proof: realm_qps,
            sync_checkpoint,
            store: store_reader,
            proof_verifier,
            job_task_store: Arc::new(task_store),
            slot_timer: SlotTimer::new(LocalClock),
            remote_latest_slot: 0,
        };
        Ok(processor)
    }

    pub async fn start(mut self) -> anyhow::Result<JoinHandle<()>> {
        info!("Realm Processor starting");
        let st = self.store.clone();
        let realm_qps = Arc::new(self.sync_proof.clone());
        let mut context: ConcreteRealmProcessorContext = RealmProcessorContext::<
            JournalStore<QEDStore>,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
        >::new(
            self.realm_config,
            st.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.proof_verifier.clone(),
        ).await?;
        info!("Realm Processor started");
        // Ensure checkpoint sync first
        self.ensure_checkpoint_sync(&mut context).await?;
        let slot_timer = self.slot_timer.clone();
        loop {
            tokio::select! {
                checkpoint_sync_result = self.ensure_checkpoint_sync(&mut context) => {
                    match checkpoint_sync_result {
                        Ok(true) => {
                            info!("Checkpoint sync completed");
                        }
                        Ok(false) => {
                            info!("No new checkpoint to sync");
                        }
                        Err(err) => {
                            error!("Checkpoint sync failed: {:?}", err);
                        }
                    }
                    continue;
                },
                slot = slot_timer.wait_for_next_slot() => {
                    info!("Next slot: {}", slot);
                }
            }

            // Build block based on slot timing
            if let Err(err) = self.validate_slot() {
                warn!("Error validating slot: {:?}", err);
                continue
            }

            let slot = self.slot_timer.get_current_slot();
            if let SlotPhase::BuildPhase(build_phase_start) = SlotPhase::get_build_phase(self.slot_timer.deref()){
                let current_timestamp = self.slot_timer.get_current_timestamp();
                if current_timestamp < build_phase_start {
                    let tt = build_phase_start - current_timestamp;
                    info!("Waiting for build phase to start: sleep {} ms, slot: {}", tt, slot);
                    tokio::time::sleep(Duration::from_millis(tt)).await;
                }
            }

            info!("Start building block");
            let proving_data_job_id: ProvingJobDataId = match self.build_block(&mut context, &realm_qps).await {
                Ok(job_id) => job_id,
                Err(err) => {
                    error!("Error building block: {:?}, slot: {}", err, slot);
                    continue;
                }
            };
            info!("Pushing job id to queue: {:?}, slot: {}", proving_data_job_id, slot);
            self.sync_proof.chq_push_imm(proving_data_job_id).await?;
            // Send the job id to the channel for the next step
            // if let Err(err) = self.queue.cdq_push_imm(proving_data_job_id).await {
            //     error!("Error chq_push_imm: {:?}", err);
            // };
            info!("Pushing job to queueue done");
        }
    }

    async fn ensure_checkpoint_sync(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<bool> {
        loop {
            match self.sync_checkpoint(context).await {
                Ok(true) => return Ok(true),  // Sync completed
                Err(err) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    bail!("Checkpoint sync attempt failed: {:?}", err)
                }
                _ => {
                    continue;
                }
            }
        }
    }

    pub async fn sync_checkpoint(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<bool> {
        let (expected_checkpoint,local_checkpoint_id) = if let Ok(local_checkpoint_id) = self.get_local_latest_l2_block_state().await {
            // Get the next expected checkpoint
            (local_checkpoint_id + 1, local_checkpoint_id)
        } else {
            (0, 0)
        };
        debug!("local_checkpoint_id {}, expected_checkpoint {}",local_checkpoint_id, expected_checkpoint);

        // Wait for the next checkpoint sync info
        let block = self.sync_checkpoint.wait_for_next_item_imm::<CheckpointSyncInfo<F>>(
            qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            expected_checkpoint
        ).await;

        match block {
            Ok(block) => {
                // checkpoint.l2_block_state
                let checkpoint_id = block.compact.l2_block_state.checkpoint_id;

                info!("Checkpoint received checkpoint_id: {},latest_checkpoint_id: {} ,local_checkpoint_id: {}", checkpoint_id, block.latest_checkpoint_id, local_checkpoint_id);
                if local_checkpoint_id >= block.latest_checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is latest");
                    self.remote_latest_slot = block.compact.slot;
                    return Ok(true);
                }
                if local_checkpoint_id >= checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is up to date");
                    return Ok(false);
                }

                info!("Syncing checkpoint");
                match context.handle_checkpoint_sync(block.compact.clone()).await {
                    Ok(_) => {
                        info!(?checkpoint_id, "Sync to new checkpoint");
                        info!("Checkpoint sync reg users: {:?}", block.compact.registered_users);
                        self.store.commit(checkpoint_id)?;
                        if local_checkpoint_id + 1 == block.latest_checkpoint_id && block.latest_checkpoint_id == checkpoint_id
                            ||  local_checkpoint_id == checkpoint_id && block.latest_checkpoint_id == checkpoint_id && local_checkpoint_id == 0
                        {
                            info!("Local checkpoint is latest");
                            self.remote_latest_slot = block.compact.slot;
                            return Ok(true);
                        }
                        Ok(false)
                    }
                    Err(err) => {
                        error!(?checkpoint_id, ?err, "Error sync checkpoint");
                        self.store.rollback(checkpoint_id)?;
                        Err(err)
                    }
                }
            }
            Err(err) => {
                error!(
                    ?local_checkpoint_id,
                    "Error getting checkpoint sync info: {:?}", err
                );
                Err(err)
            }
        }
    }

    pub async fn build_block_inner(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
        next_checkpoint_id: u64,
    ) -> anyhow::Result<ProvingJobDataId> {
        let now = Instant::now();
        context.build_block().await?;
        info!("Build block {} time: {} ms", next_checkpoint_id, now.elapsed().as_millis());
        let now = Instant::now();
        {
            let mut task_graph = context.proof_store.task_graph.lock().await;
            let sorted_tasks = task_graph.ts_task();
            self.job_task_store.save_task_topology(sorted_tasks).await?;
            task_graph.clear();
        }
        let realm_worker_output_job_id = self
            .sync_proof
            .wait_for_block_proving_jobs_imm(next_checkpoint_id)
            .await?;
        info!("Prove block {} time: {}ms", next_checkpoint_id, now.elapsed().as_millis());
        Ok(ProvingJobDataId::new(
            next_checkpoint_id,
            realm_worker_output_job_id,
        ))
    }

    pub async fn build_block(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
        realm_qps: &ProofStoreRedisAsync,
    ) -> anyhow::Result<ProvingJobDataId> {
        let local_latest_checkpoint_id = self.get_local_latest_l2_block_state().await?;
        let next_checkpoint_id = local_latest_checkpoint_id + 1;
        self.store.commit(local_latest_checkpoint_id)?;

        match self.build_block_inner(context, next_checkpoint_id).await {
            Ok(job_id) => Ok(job_id),
            Err(err) => {
                self.store.rollback(next_checkpoint_id)?;
                Err(err)
            }
        }
    }

    fn validate_slot(&self) -> anyhow::Result<()> {
        let slot = self.slot_timer.get_current_slot();
        if !self.is_current_slot() {
            bail!("Not in current slot, slot: {}, remote latest slot: {}", slot, self.remote_latest_slot)
        }

        if !self.slot_timer.is_can_reach_to_next_slot() {
            bail!("Not reach to next slot")
        }
        Ok(())
    }

    fn is_current_slot(&self) -> bool {
        self.remote_latest_slot == 0 || self.slot_timer.get_current_slot() > self.remote_latest_slot
    }
    pub async fn get_local_latest_l2_block_state(&self) -> anyhow::Result<u64> {
        let state = self
            .store
            .get_latest_l2_block_state()
            .await
            .map_err(|err| anyhow!("Error getting latest l2 block state: {:?}", err))?;
        Ok(state.checkpoint_id)
    }
}
