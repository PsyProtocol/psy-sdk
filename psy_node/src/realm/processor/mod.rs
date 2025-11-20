use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use anyhow::{anyhow, bail, ensure};
use kvq::traits::{KVQBinaryStore, KVQPair, KVQSerializable};
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    plonk::proof::ProofWithPublicInputs,
};
use psy_common::{
    data::qhashout::QHashOut,
    job::{id::QProvingJobDataID, traits::QProofStoreReaderAsync},
};
use psy_config::{
    network_constants::{GLOBAL_CONTRACT_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT},
    GenesisConfig, COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, USERS_PER_REALM,
};
use psy_crypto::{
    common::{generic_circuit_verifier::GenericCircuitVerifier, user_id::get_user_id_from_registration_id},
    hash::{
        merkle::utils::common::{QMerkleNode, SimpleMerkleNodeKey},
        traits::{hasher::MerkleZeroHasher, qhashable::QFieldHashable},
    },
};
use psy_data::{
    config::store_config::{PsyHasher, REALM_ROOT_VERSION_TABLE_TYPE, REALM_SNAPSHOT_TABLE_TYPE},
    guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput},
    models::checkpoint::sync_info::CheckpointError,
    qdata::{checkpoint::CheckpointSyncInfo, hash_key::Hash4x64Key, realm_snapshot_key::RealmSnapshotKey, user::PsyUserLeaf},
    traits::qdatastore::{qmetadata::QMetaDataStoreWriterSync, qtreedata::QTreeDataStoreWriterSync},
};
use psy_store::{
    node::realm::{PsyRealmStoreReaderAsync, PsyRealmStoreWriterAsyncImm},
    queue::{
        redis_queue::{CheckpointDrainQueueConsumerAsyncImmWithPosition, QueueOffsetState},
        task_queue::QProvingTaskStoreImpl,
        ProofStoreRedis, QPendingUserStoreAsyncImm,
    },
    store,
    store::{
        journal::{
            backup_journal::{BackupCheckpoint, BackupRealmState},
            BackupJournalStore, BackupRequest, Journal, JournalStore,
        },
        PsyStore,
    },
};
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, time::Instant};
use tracing::{error, info, trace, warn};

use super::backup::RealmS3BackupClient;
use crate::{
    common::{
        clock::SlotTimer,
        slot::{LocalClock, Slot},
        traits::realm::CoordinatorClient,
        verifier::get_cached_generic_verifier,
    },
    realm::{
        backup::{create_realm_checkpoint_backup, create_realm_pending_users_backup, create_realm_state_backup},
        client::ConcreteCoordinatorClient,
        config::RealmNodeConfig,
        state::processor::{RealmConfig, RealmConsumptionState, RealmProcessorContext},
        C, D, F,
    },
};

const CHECKPOINT_BATCH_SIZE: u64 = 25; // sync info batch size

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u64,
    pub checkpoint_id: u64,
    pub cache: Vec<u8>,
    pub root: QHashOut<F>,
    pub pre_root: QHashOut<F>,
    pub queue_offset_state: Vec<QueueOffsetState>,
}

type Context = RealmProcessorContext<
    BackupJournalStore<JournalStore<PsyStore>>,
    ProofStoreRedis,
    ProofStoreRedis,
    ProofStoreRedis,
    ProofStoreRedis,
    QProvingTaskStoreImpl,
>;

pub struct RealmProcessor {
    pub realm_config: RealmConfig,
    pub max_processed_end_caps_per_block: Option<isize>,
    pub sync_proof: ProofStoreRedis,
    pub sync_checkpoint: Arc<ProofStoreRedis>,
    pub store: PsyStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
    pub slot_timer: SlotTimer<LocalClock>,
    pub remote_latest_slot: AtomicU64,
    pub config_path: String,
    pub client: Arc<ConcreteCoordinatorClient>,
    pub backup_tx: Option<mpsc::UnboundedSender<BackupRequest>>,
    pub backup_client: Option<RealmS3BackupClient>,
    pub consumption_state: Arc<RwLock<RealmConsumptionState>>,
    pub is_state_normal: AtomicBool,
}

pub async fn run_realm_processor(config: RealmNodeConfig) -> anyhow::Result<()> {
    let realm_processor = RealmProcessor::new(config).await?;
    let _ = realm_processor.start().await?;
    Ok(())
}

impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig) -> anyhow::Result<Self> {
        info!("Realm Processor Config: {:?}", config);
        let task_store = QProvingTaskStoreImpl::new(
            &config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10),
            &config.queue.queue_biz_key,
        )
        .await?;
        let realm_qps = ProofStoreRedis::new(&config.redis.redis_uri, config.queue.queue_biz_key).await?;
        let store = store::new(&config.backend.to_backend()).await?;
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let realm_config = RealmConfig::get_standard(config.realm.realm_id);
        let sync_checkpoint = Arc::new(realm_qps.clone());

        // Initialize backup channel and task
        let (backup_tx, backup_client) = match RealmS3BackupClient::new_from_env(config.realm.realm_id).await {
            Ok(client) => {
                info!("✅ S3 backup client initialized");
                let (tx, rx) = mpsc::unbounded_channel();
                let realm_id = config.realm.realm_id;
                let backup_client = client.clone();
                tokio::spawn(async move {
                    Self::backup_task(rx, client, realm_id).await;
                });
                info!("Started realm backup task");
                (Some(tx), Some(backup_client))
            }
            Err(e) => {
                warn!("⚠️ S3 backup client initialization failed: {}", e);
                (None, None)
            }
        };

        let coordinator_client = Arc::new(ConcreteCoordinatorClient::new(config.coordinator_addr.clone())?);

        let processor = RealmProcessor {
            realm_config,
            max_processed_end_caps_per_block: config.realm.max_processed_end_caps_per_block.clone(),
            sync_proof: realm_qps,
            sync_checkpoint,
            store,
            proof_verifier,
            task_store: Arc::new(task_store),
            slot_timer: SlotTimer::new(LocalClock),
            remote_latest_slot: AtomicU64::new(0),
            config_path: config.config_path.clone(),
            client: coordinator_client,
            backup_tx,
            backup_client,
            consumption_state: Arc::new(RwLock::new(RealmConsumptionState::default())),
            is_state_normal: AtomicBool::new(true),
        };
        Ok(processor)
    }

    async fn context(&self) -> anyhow::Result<Context> {
        let realm_qps = Arc::new(self.sync_proof.clone());
        let journal_store = JournalStore::new(self.store.clone());

        // Wrap journal store with backup functionality if backup is enabled
        let backup_journal_store = if let Some(ref backup_tx) = self.backup_tx {
            BackupJournalStore::new_with_backup(journal_store, backup_tx.clone())
        } else {
            BackupJournalStore::new(journal_store)
        };

        RealmProcessorContext::<
            BackupJournalStore<JournalStore<PsyStore>>,
            ProofStoreRedis,
            ProofStoreRedis,
            ProofStoreRedis,
            ProofStoreRedis,
            QProvingTaskStoreImpl,
        >::new_with_state(
            self.realm_config,
            self.max_processed_end_caps_per_block,
            backup_journal_store,
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.task_store.clone(),
            self.proof_verifier.clone(),
            self.consumption_state.clone(),
        )
        .await
    }

    async fn get_realm_root_diff(&self, build_ctx: &Context, checkpoint_id: u64) -> anyhow::Result<(QHashOut<F>, QHashOut<F>)> {
        let pre_realm_root = self.get_realm_root(&self.store, checkpoint_id.saturating_sub(1)).await?;
        let realm_root = self.get_realm_root(&build_ctx.store, checkpoint_id).await?;
        Ok((pre_realm_root, realm_root))
    }

    async fn get_realm_root(&self, store: &dyn PsyRealmStoreReaderAsync<F>, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        let realm_root = store
            .get_user_sub_tree_merkle_proof(checkpoint_id, 0, COORDINATOR_USER_TREE_HEIGHT, self.realm_config.realm_id as u64)
            .await?;
        Ok(realm_root.value)
    }

    pub async fn start(self) -> anyhow::Result<()> {
        info!("Realm Processor starting");
        self.initialize().await?;
        self.run().await?;
        Ok(())
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        if let Err(e) = self.store.get_latest_block_state().await {
            warn!("Failed to get latest L2 block state: {:?}", e);
            if let Ok(CheckpointError::NotFound) = e.downcast::<CheckpointError>() {
                let ctx = self.context().await?;
                // initialize genesis state
                self.initialize_genesis_state(&ctx).await?;
                let block0 = self.client.get_checkpoint_sync_info(self.realm_config.realm_id, 0).await?;
                self.handle_sync_info(&ctx, block0).await?;
            } else {
                self.is_state_normal.store(false, Ordering::Relaxed);
            }
        }

        // recover from backup if available
        self.recover_from_backup().await?;

        Ok(())
    }

    async fn recover_from_backup(&self) -> anyhow::Result<()> {
        let ctx = self.context().await?;
        if let Some(backup_client) = &self.backup_client {
            let recovery_info = match backup_client.fetch_recovery_info().await {
                Ok(recovery_info) => recovery_info,
                Err(e) => {
                    warn!("Failed to fetch recovery info from backup: {:?}, may be backup is not available", e);
                    return Ok(());
                }
            };
            info!(
                "📋 Realm {} recovery info: latest checkpoint {}, available: {:?}",
                backup_client.realm_id, recovery_info.latest_checkpoint, recovery_info.checkpoints_available
            );

            let local_latest_checkpoint_id = self.get_local_latest_checkpoint_id().await?;
            let global_latest_checkpoint_id = self
                .client
                .get_latest_checkpoint_sync_info(self.realm_config.realm_id)
                .await?
                .latest_checkpoint_id;
            info!(
                "Local latest checkpoint {} vs backup latest checkpoint {}",
                local_latest_checkpoint_id, recovery_info.latest_checkpoint
            );
            info!(
                "Local latest checkpoint {} vs global latest checkpoint {}",
                local_latest_checkpoint_id, global_latest_checkpoint_id
            );

            let target_checkpoint_id = std::cmp::min(global_latest_checkpoint_id, recovery_info.latest_checkpoint);

            if local_latest_checkpoint_id >= target_checkpoint_id && local_latest_checkpoint_id <= global_latest_checkpoint_id {
                info!(
                    "Local latest checkpoint {} is greater than target checkpoint {}, no need to recover",
                    local_latest_checkpoint_id, target_checkpoint_id
                );
            } else {
                let from_checkpoint_id = if local_latest_checkpoint_id >= target_checkpoint_id {
                    info!(
                        "Local latest checkpoint {} is greater than target checkpoint {}, recover from checkpoint 0",
                        local_latest_checkpoint_id, target_checkpoint_id
                    );
                    0
                } else {
                    info!(
                        "Local latest checkpoint {} is less than target checkpoint {}, recover from local checkpoint {}",
                        local_latest_checkpoint_id, target_checkpoint_id, local_latest_checkpoint_id
                    );
                    local_latest_checkpoint_id
                };
                info!(
                    "Recovering from backup from checkpoint {} to {}",
                    from_checkpoint_id, target_checkpoint_id
                );

                // check state
                let current_sync_info = &self
                    .client
                    .get_checkpoint_sync_info(self.realm_config.realm_id, from_checkpoint_id)
                    .await?;
                let local_realm_root = ctx
                    .store
                    .get_user_sub_tree_merkle_proof(
                        from_checkpoint_id,
                        COORDINATOR_USER_TREE_HEIGHT,
                        COORDINATOR_USER_TREE_HEIGHT,
                        self.realm_config.realm_id as u64,
                    )
                    .await?
                    .value;
                ensure!(
                    current_sync_info.realm_root == local_realm_root,
                    "Realm root mismatch, remote realm root: {}, local realm root: {}",
                    current_sync_info.realm_root,
                    local_realm_root
                );

                let start_realm_state_backup = backup_client.fetch_realm_state(from_checkpoint_id).await.unwrap_or_default();
                let realm_state_backup = backup_client.fetch_realm_state(target_checkpoint_id).await?;
                trace!("start realm state: {}", serde_json::to_string_pretty(&start_realm_state_backup)?);
                trace!("end realm state: {}", serde_json::to_string_pretty(&realm_state_backup)?);
                {
                    let mut realm_state = ctx
                        .consumption_state
                        .write()
                        .map_err(|_| anyhow::anyhow!("Failed to acquire write lock"))?;
                    *realm_state = RealmConsumptionState {
                        last_processed_user_num: start_realm_state_backup.last_processed_user_num,
                        current_processed_user_num: start_realm_state_backup.current_processed_user_num,
                        total_pending_users: start_realm_state_backup.total_pending_users,
                    };
                }

                info!("Cleanup pending users");
                // handle consumption state
                let count = ctx.sync_queue.get_pending_users_count().await?;
                if count != 0 {
                    warn!("Sync queue has {} pending users", count);
                    let (local_pending_users, _) =
                        QPendingUserStoreAsyncImm::peek_with_position::<F>(&*ctx.sync_queue, count, from_checkpoint_id).await?;
                    let start = if start_realm_state_backup.is_committed {
                        start_realm_state_backup.current_processed_user_num
                    } else {
                        start_realm_state_backup.last_processed_user_num
                    };
                    let remote_pending_users = backup_client
                        .fetch_pending_users(start, start_realm_state_backup.total_pending_users)
                        .await?;
                    if local_pending_users != remote_pending_users {
                        let pop_pending_users = ctx.sync_queue.pop_pending_users::<F>(count).await?;
                        ctx.sync_queue.push_pending_users(&remote_pending_users).await?;
                        trace!(
                            "Popped {} pending users from sync queue: {}",
                            count,
                            serde_json::to_string_pretty(&pop_pending_users)?
                        );
                    }
                }

                // let sync_infos = self.fetch_remote_sync_infos(local_latest_checkpoint_id,
                // recovery_info.latest_checkpoint).await?;
                for checkpoint_id in from_checkpoint_id + 1..=target_checkpoint_id {
                    // handle sync info
                    // let current_sync_info = sync_infos[checkpoint_id as usize -
                    // local_latest_checkpoint_id as usize].clone();
                    let current_sync_info = &self.client.get_checkpoint_sync_info(self.realm_config.realm_id, checkpoint_id).await?;

                    self.handle_sync_info(&ctx, current_sync_info.clone()).await?;

                    if !recovery_info.checkpoints_available.contains(&checkpoint_id) {
                        warn!("⚠️ Checkpoint {} not available in S3, skipping", checkpoint_id);
                        continue;
                    }

                    let backup = backup_client.fetch_checkpoint_backup(checkpoint_id).await?;
                    if !backup.is_committed {
                        warn!("⚠️ Checkpoint {} not commited in S3, skipping", checkpoint_id);
                        continue;
                    }

                    info!(
                        "📝 Applying {} journal changes and {} removed keys for realm {} checkpoint {}",
                        backup.pair_to_set.len(),
                        backup.removed_keys.len(),
                        self.realm_config.realm_id,
                        checkpoint_id
                    );

                    let pair_to_set = backup
                        .pair_to_set
                        .iter()
                        .map(|(k, v)| KVQPair {
                            key: k.clone(),
                            value: v.clone(),
                        })
                        .collect::<Vec<_>>();

                    let pair_to_set_ref: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = pair_to_set
                        .iter()
                        .map(|kv| KVQPair {
                            key: &kv.key,
                            value: &kv.value,
                        })
                        .collect();
                    let removed_keys = &backup.removed_keys;
                    ctx.store.set_and_delete_many(&pair_to_set_ref, &removed_keys)?;
                    ctx.store.commit(None).await?;

                    let local_realm_root = ctx
                        .store
                        .get_user_sub_tree_merkle_proof(
                            checkpoint_id,
                            COORDINATOR_USER_TREE_HEIGHT,
                            COORDINATOR_USER_TREE_HEIGHT,
                            self.realm_config.realm_id as u64,
                        )
                        .await?
                        .value;
                    info!(
                        "Remote realm root: {}, local realm root: {}",
                        current_sync_info.realm_root, local_realm_root
                    );
                    ensure!(
                        current_sync_info.realm_root == local_realm_root,
                        "Realm root mismatch, remote realm root: {}, local realm root: {}",
                        current_sync_info.realm_root,
                        local_realm_root
                    );
                }

                {
                    let mut realm_state = ctx
                        .consumption_state
                        .write()
                        .map_err(|_| anyhow::anyhow!("Failed to acquire write lock"))?;
                    realm_state.last_processed_user_num = realm_state_backup.last_processed_user_num;
                    realm_state.current_processed_user_num = realm_state_backup.current_processed_user_num;
                }

                // handle pending users
                // let pending_users_count = ctx.sync_queue.get_pending_users_count().await? as
                // u64; if pending_users_count +
                // start_realm_state_backup.last_processed_user_num !=
                // realm_state_backup.total_pending_users {     error!("Pending
                // users count {} + last_processed_user_num {} not equal to backup pending users
                // count {}", pending_users_count,
                // start_realm_state_backup.last_processed_user_num,
                // realm_state_backup.total_pending_users);     anyhow::bail!("
                // Pending users count {} + last_processed_user_num {} not equal to backup
                // pending users count {}", pending_users_count,
                // start_realm_state_backup.last_processed_user_num,
                // realm_state_backup.total_pending_users); }
                if realm_state_backup.total_pending_users == 0 {
                    info!("No pending users, skip fetching pending users");
                }

                let global_current_processed_user_num = if realm_state_backup.is_committed {
                    realm_state_backup.current_processed_user_num
                } else {
                    realm_state_backup.last_processed_user_num
                };
                let processed_pending_users_num = global_current_processed_user_num - start_realm_state_backup.last_processed_user_num;
                info!(
                    "Global current processed user num: {}, processed pending users num: {}",
                    global_current_processed_user_num, processed_pending_users_num
                );
                info!(
                    "Popping {} users which are already processed from sync queue",
                    processed_pending_users_num
                );
                if processed_pending_users_num != 0 {
                    let poped_users = ctx.sync_queue.pop_pending_users::<F>(processed_pending_users_num as usize).await?;
                    trace!("Popping users: {}", serde_json::to_string_pretty(&poped_users)?);
                }

                let pending_users_left_num = ctx.sync_queue.get_pending_users_count().await? as u64;

                if pending_users_left_num != 0 {
                    info!("Fetching {} pending users from backup", pending_users_left_num);
                    let pending_users = backup_client
                        .fetch_pending_users(
                            global_current_processed_user_num,
                            global_current_processed_user_num + pending_users_left_num,
                        )
                        .await?;
                    let (peek_pending_users, _) = QPendingUserStoreAsyncImm::peek_with_position(
                        &*ctx.sync_queue,
                        pending_users_left_num as usize,
                        recovery_info.latest_checkpoint,
                    )
                    .await?;
                    trace!("Pending users: {}", serde_json::to_string_pretty(&pending_users)?);
                    trace!("Peek pending users: {}", serde_json::to_string_pretty(&peek_pending_users)?);
                    ensure!(
                        peek_pending_users == pending_users,
                        "Peek pending users not equal to backup pending users"
                    );
                } else {
                    info!("No pending users left, skip fetching pending users");
                }

                // cleanup endcaps
                let count = ctx.checkpoint_queue.get_queue_count(ctx.realm_config.guta_channel_id).await?;
                if count != 0 {
                    warn!("Endcap queue has {} endcaps", count);
                    let endcap_queue_state = QueueOffsetState {
                        start_position: 0,
                        end_position: count as i64,
                        checkpoint_id: target_checkpoint_id,
                        channel_id: ctx.realm_config.guta_channel_id,
                        consumed_count: count as usize,
                    };
                    CheckpointDrainQueueConsumerAsyncImmWithPosition::commit_offset(&*ctx.checkpoint_queue, &endcap_queue_state).await?;
                    warn!("Popped {} endcaps from checkpoint queue", count);
                }

                // recover checkpoint cache
                let latest_cached_checkpoint = backup_client.find_last_available_cached_checkpoint(global_latest_checkpoint_id + 1)?;
                info!("Latest cached checkpoint: {}", latest_cached_checkpoint);
                if backup_client.find_last_available_checkpoint(latest_cached_checkpoint)? >= realm_state_backup.checkpoint_id {
                    if let Ok(cache) = backup_client.fetch_checkpoint_backup(latest_cached_checkpoint).await {
                        info!("Checkpoint backup cache {} found", latest_cached_checkpoint);
                        if cache.is_committed {
                            info!("Checkpoint backup cache {} is committed, skip recovering", latest_cached_checkpoint);
                            return Ok(());
                        }
                        let pair_to_set = cache
                            .pair_to_set
                            .iter()
                            .map(|(k, v)| KVQPair {
                                key: k.clone(),
                                value: v.clone(),
                            })
                            .collect::<Vec<_>>();

                        let pair_to_set_ref: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = pair_to_set
                            .iter()
                            .map(|kv| KVQPair {
                                key: &kv.key,
                                value: &kv.value,
                            })
                            .collect();
                        ctx.store.set_and_delete_many(&pair_to_set_ref, &cache.removed_keys)?;
                        let _ = self.save_snapshot(&ctx, latest_cached_checkpoint, vec![]).await?;

                        let local_realm_root = ctx
                            .store
                            .get_user_sub_tree_merkle_proof(
                                latest_cached_checkpoint,
                                COORDINATOR_USER_TREE_HEIGHT,
                                COORDINATOR_USER_TREE_HEIGHT,
                                self.realm_config.realm_id as u64,
                            )
                            .await?
                            .value;
                        trace!("Local realm root: {}", local_realm_root);

                        // recover pending user state
                        if let Ok(queue_state) = backup_client.fetch_pending_users_queue_state(latest_cached_checkpoint).await {
                            info!("Queue state backup {} found", latest_cached_checkpoint);
                            ctx.sync_queue.set_last_peek_offset(&queue_state).await?;
                        } else {
                            info!("No queue state at checkpoint {}", latest_cached_checkpoint);
                        }
                    } else {
                        info!("No checkpoint cache {} found", latest_cached_checkpoint);
                    }
                } else {
                    info!("No commit cache");
                }
            }
        } else {
            warn!("Backup client is None, cannot recover from backup");
        }

        Ok(())
    }

    async fn run(&self) -> anyhow::Result<()> {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Err(err) = self.build().await {
                error!("Build error: {:?}", err);
            }
        }
    }

    async fn confirm_checkpoint(&self, realm_root: QHashOut<F>, checkpoint_id: u64, latest_checkpoint_id: u64) -> anyhow::Result<()> {
        let snapshot = self.load_snapshot(realm_root)?;
        let build_ctx = self.context().await?;
        build_ctx.store.restore_cache(snapshot.cache).await?;
        trace!(
            "synced checkpoint id: {}, latest checkpoint id: {}, realm root: {}",
            checkpoint_id,
            latest_checkpoint_id,
            realm_root
        );
        build_ctx
            .commit(checkpoint_id, snapshot.queue_offset_state)
            .await
            .map_err(|e| anyhow!("Failed to commit checkpoint {}: {}", checkpoint_id, e))?;
        info!("Commit checkpoint {}, latest_checkpoint_id: {}", checkpoint_id, latest_checkpoint_id);
        Ok(())
    }

    async fn build(&self) -> anyhow::Result<()> {
        if !self.is_state_normal.load(Ordering::Relaxed) {
            warn!("State is not normal, skipping");
            return Ok(());
        }

        self.sync_to_latest().await?;

        if let Err(err) = self.validate_slot() {
            warn!("Error validating slot: {:?}", err);
            return Ok(());
        }

        let slot = self.slot_timer.get_current_slot();
        trace!("Next slot: {}", slot);
        let local_latest_checkpoint_id = self.get_local_latest_checkpoint_id().await?;
        let next_checkpoint_id = local_latest_checkpoint_id + 1;
        let build_ctx = &self.context().await?;
        let has_tasks = build_ctx.has_pending_guta_tasks(next_checkpoint_id).await? || build_ctx.has_pending_user_tasks().await?;
        if !has_tasks {
            trace!("No pending tasks for checkpoint {}, skipping block construction", next_checkpoint_id);
            return Ok(());
        }
        let now = Instant::now();
        info!("Start building block checkpoint: {}, slot: {}", next_checkpoint_id, slot);
        match self.build_block(build_ctx, next_checkpoint_id, slot).await {
            Ok((job_id, off_state)) => {
                let (pair_to_set, remove_keys) = self.save_snapshot(build_ctx, next_checkpoint_id, off_state).await?;
                self.submit_guta(build_ctx, job_id).await?;
                // backup commit cache
                if let Some(backup_tx) = &self.backup_tx {
                    let pair_to_set = pair_to_set.into_iter().map(|pair| (pair.key, pair.value)).collect();
                    let request = BackupCheckpoint {
                        checkpoint_id: next_checkpoint_id,
                        pair_to_set,
                        removed_keys: remove_keys,
                        is_committed: false,
                    };
                    if let Err(e) = backup_tx.send(BackupRequest::Checkpoint(request)) {
                        error!("❌ Failed to send backup request for checkpoint {}: {}", next_checkpoint_id, e);
                    }
                    // backup realm state
                    {
                        let realm_state = build_ctx.consumption_state.read().map_err(|_| anyhow::anyhow!(""))?;
                        let backup_realm_state = BackupRealmState {
                            checkpoint_id: next_checkpoint_id,
                            last_processed_user_num: realm_state.last_processed_user_num,
                            current_processed_user_num: realm_state.current_processed_user_num,
                            total_pending_users: realm_state.total_pending_users,
                            is_committed: false,
                        };
                        if let Err(e) = backup_tx.send(BackupRequest::RealmState(backup_realm_state)) {
                            error!("❌ Failed to send backup request for checkpoint {}: {}", next_checkpoint_id, e);
                        }
                    }
                    // backup pending user state
                    if let Some(pending_user_queue_state) = QPendingUserStoreAsyncImm::get_last_peek_offset(&*build_ctx.sync_queue).await? {
                        if let Err(e) = backup_tx.send(BackupRequest::PendingUsersQueueState(pending_user_queue_state)) {
                            error!("❌ Failed to send backup request for checkpoint {}: {}", next_checkpoint_id, e);
                        }
                    }
                } else {
                    warn!("Backup tx is None, cannot backup checkpoint cache {}", next_checkpoint_id);
                }
                info!(
                    "Build complete checkpoint: {}, slot: {}, cost time: {:?}",
                    next_checkpoint_id,
                    slot,
                    now.elapsed()
                );
            }
            Err(err) => {
                // Rollback database changes
                build_ctx.rollback(next_checkpoint_id).await?;
                error!("Rollback: build block failed for checkpoint {}: {:?}", next_checkpoint_id, err);
            }
        }
        Ok(())
    }

    async fn save_snapshot(
        &self,
        build_ctx: &Context,
        checkpoint_id: u64,
        offset_state: Vec<QueueOffsetState>,
    ) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let (pre_realm_root, realm_root) = self.get_realm_root_diff(build_ctx, checkpoint_id).await?;

        if let Some(cache) = build_ctx.store.get_cache().await? {
            // Get or initialize version
            let version = self.get_realm_root_version(realm_root)?;

            let snapshot = Snapshot {
                version,
                checkpoint_id,
                cache,
                root: realm_root.clone(),
                pre_root: pre_realm_root,
                queue_offset_state: offset_state,
            };

            // Serialize data
            let snapshot_value = bincode::serialize(&snapshot)?;
            let version_value = bincode::serialize(&version)?;

            // Use structured keys
            let snapshot_key = self.snapshot_key(realm_root, version)?;
            let version_key = self.version_key(realm_root)?;

            let set = vec![
                KVQPair {
                    key: &version_key,
                    value: &version_value,
                },
                KVQPair {
                    key: &snapshot_key,
                    value: &snapshot_value,
                },
            ];
            self.store.set_and_delete_many(&set, &*vec![])?;
            let pair_to_set = set
                .into_iter()
                .map(|pair| KVQPair {
                    key: pair.key.clone(),
                    value: pair.value.clone(),
                })
                .collect();
            Ok((pair_to_set, vec![]))
        } else {
            Ok((vec![], vec![]))
        }
    }

    fn load_snapshot(&self, realm_root: QHashOut<F>) -> anyhow::Result<Snapshot> {
        // Get version
        let version_key = self.version_key(realm_root)?;
        let version_data = self
            .store
            .get_exact_if_exists(&version_key)?
            .ok_or_else(|| anyhow!("Snapshot version not found for realm root: {}", realm_root))?;
        let version: u64 = bincode::deserialize(&version_data)?;

        // Get snapshot
        let snapshot_key = self.snapshot_key(realm_root, version)?;
        let snapshot_data = self
            .store
            .get_exact_if_exists(&snapshot_key)?
            .ok_or_else(|| anyhow!("Snapshot data not found for realm root: {}, version: {}", realm_root, version))?;
        let snapshot: Snapshot = bincode::deserialize(&snapshot_data)?;
        Ok(snapshot)
    }

    fn snapshot_key(&self, realm_root: QHashOut<F>, version: u64) -> anyhow::Result<Vec<u8>> {
        let snapshot_key = RealmSnapshotKey::<REALM_SNAPSHOT_TABLE_TYPE>::new(realm_root, version).to_bytes()?;
        Ok(snapshot_key)
    }

    fn version_key(&self, realm_root: QHashOut<F>) -> anyhow::Result<Vec<u8>> {
        let version_key = Hash4x64Key::<REALM_ROOT_VERSION_TABLE_TYPE>::from(realm_root).to_bytes()?;
        Ok(version_key)
    }

    fn is_candidate(&self, realm_root: QHashOut<F>) -> anyhow::Result<bool> {
        let version_key = self.version_key(realm_root)?;
        let version = self.store.get_exact_if_exists(&version_key)?;
        Ok(version.is_some())
    }

    fn get_realm_root_version(&self, realm_root: QHashOut<F>) -> anyhow::Result<u64> {
        let version_key = self.version_key(realm_root)?;
        match self.store.get_exact_if_exists(&version_key)? {
            Some(data) => {
                let version: u64 = bincode::deserialize(&data)?;
                Ok(version + 1)
            }
            None => Ok(0),
        }
    }

    async fn submit_guta(&self, build_ctx: &Context, job_id: QProvingJobDataID) -> anyhow::Result<()> {
        let bytes = build_ctx.proof_store.get_bytes_by_id(job_id).await?;
        // Deserialize realm result
        let realm_result: GUTARealmCheckpointResult<F> = bincode::deserialize(&bytes)?;
        // Get proof with retry
        let proof: ProofWithPublicInputs<F, C, D> = build_ctx.proof_store.get_proof_by_id(realm_result.proof_id.get_output_id()).await?;
        // let proof = self.get_proof_with_retry(proof_store,
        // realm_result.proof_id.get_output_id()).await?;
        let input = SubmitGUTARealmResultAPINoProofInput::<F> {
            realm_id: self.realm_config.realm_id as u64,
            checkpoint_id: realm_result.checkpoint_id,
            guta_stats: realm_result.guta_stats,
            top_line_proof: realm_result.top_line_proof,
            checkpoint_tree_root: realm_result.checkpoint_tree_root,
            proof_id: realm_result.proof_id.get_output_id(),
        };
        self.client.submit_guta_v1(&input, &bincode::serialize(&proof)?, input.realm_id).await
    }

    async fn sync_to_latest(&self) -> anyhow::Result<()> {
        loop {
            let has_pending_guta = self.client.has_pending_guta(self.realm_config.realm_id as u32).await?;
            if has_pending_guta {
                warn!("Has pending guta, skipping sync to latest");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            loop {
                let local_checkpoint_id = self.get_local_latest_checkpoint_id().await?;
                let latest_sync_info = self.get_remote_latest_checkpoint_sync_info().await?;
                if local_checkpoint_id == latest_sync_info.latest_checkpoint_id {
                    info!("Local checkpoint is up to date");
                    self.remote_latest_slot.store(latest_sync_info.compact.slot, Ordering::Relaxed);
                    return Ok(());
                }
                let start = local_checkpoint_id;
                let end = local_checkpoint_id + CHECKPOINT_BATCH_SIZE;
                self.sync_checkpoint_range(start, end).await?;
            }
        }
    }

    pub async fn handle_sync_info(&self, sync_ctx: &Context, block: CheckpointSyncInfo<F>) -> anyhow::Result<()> {
        let sync_checkpoint_id = block.compact.block_state.checkpoint_id;
        match sync_ctx.handle_checkpoint_sync(block.compact.clone()).await {
            Ok(_) => {
                info!(
                    "Checkpoint {} sync reg users len: {}",
                    sync_checkpoint_id,
                    block.compact.registered_users.len()
                );

                let pending_users_count = sync_ctx.sync_queue.get_pending_users_count().await?;
                trace!("Pending users count after checkpoint sync: {}", pending_users_count);
                sync_ctx.store.commit(None).await?;
                Ok(())
            }
            Err(err) => {
                error!(?sync_checkpoint_id, ?err, "Error sync checkpoint");
                sync_ctx.store.rollback(sync_checkpoint_id).await?;
                Err(err)
            }
        }
    }

    async fn sync_checkpoint_range(&self, start: u64, end: u64) -> anyhow::Result<()> {
        // [start, end)
        let mut sync_infos = self
            .fetch_remote_sync_infos(start, end)
            .await
            .map_err(|e| anyhow!("Fetch remote sync infos error: {:?}", e))?;
        if sync_infos.is_empty() {
            return Ok(());
        }
        let start_sync_info = sync_infos.remove(0);
        let expected_realm_root = start_sync_info.realm_root;
        let mut local_checkpoint_id = start_sync_info.compact.block_state.checkpoint_id;
        let mut local_realm_root_from_store = self.get_realm_root(&self.store, start).await?;
        if expected_realm_root != local_realm_root_from_store {
            self.is_state_normal.store(false, Ordering::Relaxed);
            return Err(anyhow!(
                "Realm root mismatch: expected_realm_root {} != local_realm_root_from_store {}, checkpoint_id: {}",
                expected_realm_root,
                local_realm_root_from_store,
                start
            ));
        }

        for block in sync_infos {
            let sync_checkpoint_id = block.compact.block_state.checkpoint_id;
            let latest_checkpoint_id = block.latest_checkpoint_id;
            let remote_realm_root = block.realm_root;
            info!("Checkpoint received checkpoint_id: {},latest_checkpoint_id: {} ,local_checkpoint_id: {}, local_realm_root: {}, remote_realm_root: {}",
                sync_checkpoint_id, latest_checkpoint_id, local_checkpoint_id, local_realm_root_from_store, remote_realm_root);
            self.handle_sync_info(&self.context().await?, block)
                .await
                .map_err(|e| anyhow!("Handle sync info attempt failed: {:?}", e))?;
            // confirm pending checkpoint
            if local_realm_root_from_store != remote_realm_root && self.is_candidate(remote_realm_root.clone())? {
                self.confirm_checkpoint(remote_realm_root.clone(), sync_checkpoint_id, latest_checkpoint_id)
                    .await?;
                local_realm_root_from_store = remote_realm_root;
            }
            local_checkpoint_id = sync_checkpoint_id;
        }

        Ok(())
    }

    async fn get_remote_latest_checkpoint_sync_info(&self) -> anyhow::Result<CheckpointSyncInfo<F>> {
        self.client.get_latest_checkpoint_sync_info(self.realm_config.realm_id).await
    }

    async fn fetch_remote_sync_infos(&self, start: u64, end: u64) -> anyhow::Result<Vec<CheckpointSyncInfo<F>>> {
        self.client
            .get_latest_block_updates_from_coordinator(self.realm_config.realm_id as u64, start, end)
            .await
    }

    pub async fn build_block(
        &self,
        build_ctx: &Context,
        next_checkpoint_id: u64,
        slot: u64,
    ) -> anyhow::Result<(QProvingJobDataID, Vec<QueueOffsetState>)> {
        build_ctx.build_block(next_checkpoint_id, slot).await
    }

    fn validate_slot(&self) -> anyhow::Result<()> {
        let slot = self.slot_timer.get_current_slot();
        if !self.is_current_slot() {
            bail!(
                "Not in current slot, slot: {}, remote latest slot: {}",
                slot,
                self.remote_latest_slot.load(Ordering::Relaxed)
            )
        }
        Ok(())
    }

    fn is_current_slot(&self) -> bool {
        self.remote_latest_slot.load(Ordering::Relaxed) == 0 || self.slot_timer.get_current_slot() >= self.remote_latest_slot.load(Ordering::Relaxed)
    }
    pub async fn get_local_latest_checkpoint_id(&self) -> anyhow::Result<u64> {
        let state = self
            .store
            .get_latest_block_state()
            .await
            .map_err(|err| anyhow!("Error getting latest l2 block state: {:?}", err))?;
        Ok(state.checkpoint_id)
    }

    pub async fn initialize_store(
        store: Arc<dyn KVQBinaryStore>,
        genesis_config: Option<GenesisConfig<GoldilocksField>>,
        realm_id: u32,
    ) -> anyhow::Result<()> {
        if let Some(genesis_config) = genesis_config {
            info!("Processing genesis state for realm {}", realm_id);

            let realm_start_user = (realm_id as u64) * USERS_PER_REALM;
            let realm_end_user = ((realm_id + 1) as u64) * USERS_PER_REALM;

            let mut user_contract_states: HashMap<u64, HashMap<u64, Vec<(u64, QHashOut<F>)>>> = HashMap::new();
            let mut user_id_to_register_id: HashMap<u64, u64> = HashMap::new();

            for (contract_id, users) in genesis_config.get_all_contracts() {
                for (register_id, user_state) in users {
                    let user_id = get_user_id_from_registration_id(*register_id);

                    if user_id >= realm_start_user && user_id < realm_end_user {
                        user_id_to_register_id.insert(user_id, *register_id);

                        let mut contract_slots = Vec::new();
                        for (slot_id, slot_value) in &user_state.slots {
                            contract_slots.push((*slot_id, *slot_value));
                        }

                        if !contract_slots.is_empty() {
                            user_contract_states
                                .entry(user_id)
                                .or_insert_with(HashMap::new)
                                .insert(*contract_id, contract_slots);
                        }
                    }
                }
            }

            let genesis_users = genesis_config.get_genesis_users();

            let mut realm_updates = Vec::new();
            for (user_id, contracts) in user_contract_states {
                let register_id = user_id_to_register_id[&user_id];

                let mut user_contract_tree_root = PsyHasher::get_zero_hash(GLOBAL_CONTRACT_TREE_HEIGHT.into());

                for (contract_id, slots) in contracts {
                    let mut contract_state_root = PsyHasher::get_zero_hash(MAX_CONTRACT_STATE_TREE_HEIGHT.into());
                    for (slot_id, slot_value) in slots {
                        contract_state_root = store
                            .set_user_state_tree_leaf_hash(0, user_id, contract_id as u32, MAX_CONTRACT_STATE_TREE_HEIGHT, slot_id, slot_value)?
                            .new_root;
                    }

                    user_contract_tree_root = store
                        .set_user_contract_tree_leaf_hash(0, user_id, contract_id as u32, contract_state_root)?
                        .new_root;
                }

                let user_leaf = PsyUserLeaf {
                    public_key: genesis_users[register_id as usize].qfhash::<PsyHasher>(),
                    user_state_tree_root: user_contract_tree_root,
                    balance: F::ZERO,
                    nonce: F::ZERO,
                    last_checkpoint_id: F::ZERO,
                    event_index: F::ZERO,
                    user_id: F::from_canonical_u64(user_id),
                };

                let user_leaf_hash = user_leaf.qfhash::<PsyHasher>();

                store
                    .set_user_leaf_data(0, &user_leaf)
                    .map_err(|e| anyhow::anyhow!("Failed to set user leaf data for user {}: {}", user_id, e))?;

                let realm_update = QMerkleNode {
                    key: SimpleMerkleNodeKey {
                        level: GLOBAL_USER_TREE_HEIGHT,
                        index: user_id,
                    },
                    value: user_leaf_hash,
                };
                realm_updates.push(realm_update);

                info!(
                    "✅ Genesis state set for user {} with UCT root {} (realm {})",
                    user_id, user_contract_tree_root, realm_id
                );
            }

            if !realm_updates.is_empty() {
                store.injest_user_tree_nodes_imm(0, COORDINATOR_USER_TREE_HEIGHT, &realm_updates).await?;
                info!(
                    "✅ Genesis state initialization completed for realm {} with {} users",
                    realm_id,
                    realm_updates.len()
                );
            } else {
                info!("⚠️ Genesis state initialization skipped for realm {} (no users assigned)", realm_id);
            }
        }

        Ok(())
    }

    async fn initialize_genesis_state(&self, ctx: &Context) -> anyhow::Result<()> {
        let config = psy_config::PsyConfigGoldilocks::from_file("config.json")?;
        let network = config.get_current_network()?;
        let genesis_config = if let Some(genesis) = &network.genesis {
            let genesis_json = serde_json::to_string(genesis)?;
            Some(GenesisConfig::from_json(&genesis_json)?)
        } else {
            None
        };
        let store = ctx.store.clone();
        Self::initialize_store(Arc::new(store), genesis_config, self.realm_config.realm_id).await
    }

    async fn backup_task(mut rx: mpsc::UnboundedReceiver<BackupRequest>, backup_client: RealmS3BackupClient, realm_id: u32) {
        info!("🚀 Realm backup task started");
        while let Some(request) = rx.recv().await {
            for retry_count in 0..=3 {
                // Retry up to 3 times with 1 second delay
                match request {
                    BackupRequest::Checkpoint(ref checkpoint_req) => {
                        let backup = create_realm_checkpoint_backup(
                            realm_id,
                            checkpoint_req.checkpoint_id,
                            checkpoint_req.pair_to_set.clone(),
                            checkpoint_req.removed_keys.clone(),
                            checkpoint_req.is_committed,
                        );
                        match backup_client.backup_checkpoint(&backup).await {
                            Ok(_) => {
                                info!("✅ Realm checkpoint {} backup succeeded", checkpoint_req.checkpoint_id);
                                break;
                            }
                            Err(e) if retry_count < 3 => {
                                warn!(
                                    "⚠️ Realm backup retry {}/3 for checkpoint {}: {}",
                                    retry_count + 1,
                                    checkpoint_req.checkpoint_id,
                                    e
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                            Err(e) => {
                                error!("❌ Realm backup final failure for checkpoint {}: {}", checkpoint_req.checkpoint_id, e);
                            }
                        }
                    }
                    BackupRequest::PendingUsers(ref pending_users_req) => {
                        let backup = create_realm_pending_users_backup(
                            realm_id,
                            pending_users_req.checkpoint_id,
                            pending_users_req.start_user_index,
                            pending_users_req.pending_users.clone(),
                        );
                        match backup_client.backup_pending_users(&backup).await {
                            Ok(_) => {
                                info!(
                                    "✅ Realm pending {} users from {} backup at checkpoint {} succeeded",
                                    pending_users_req.pending_users.len(),
                                    pending_users_req.start_user_index,
                                    pending_users_req.checkpoint_id
                                );
                                break;
                            }
                            Err(e) if retry_count < 3 => {
                                warn!(
                                    "⚠️ Realm backup retry {}/3 for pending users {}: {}",
                                    retry_count + 1,
                                    pending_users_req.checkpoint_id,
                                    e
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                            Err(e) => {
                                error!(
                                    "❌ Realm backup final failure for pending users {}: {}",
                                    pending_users_req.checkpoint_id, e
                                );
                            }
                        }
                    }
                    BackupRequest::PendingUsersQueueState(ref pending_users_queue_state_req) => {
                        match backup_client.backup_pending_users_queue_state(pending_users_queue_state_req).await {
                            Ok(_) => {
                                info!(
                                    "✅ Realm pending users queue state {}",
                                    serde_json::to_string_pretty(&pending_users_queue_state_req).unwrap()
                                );
                                break;
                            }
                            Err(e) if retry_count < 3 => {
                                warn!(
                                    "⚠️ Realm backup retry {}/3 for pending users {}: {}",
                                    retry_count + 1,
                                    pending_users_queue_state_req.checkpoint_id,
                                    e
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                            Err(e) => {
                                error!(
                                    "❌ Realm backup final failure for pending users {}: {}",
                                    pending_users_queue_state_req.checkpoint_id, e
                                );
                            }
                        }
                    }
                    BackupRequest::RealmState(ref realm_state_req) => {
                        let backup = create_realm_state_backup(
                            realm_id,
                            realm_state_req.checkpoint_id,
                            realm_state_req.last_processed_user_num,
                            realm_state_req.current_processed_user_num,
                            realm_state_req.total_pending_users,
                            realm_state_req.is_committed,
                        );
                        match backup_client.backup_realm_state(&backup).await {
                            Ok(_) => {
                                info!("✅ Realm realm state backup {} backup succeeded", realm_state_req.checkpoint_id);
                                break;
                            }
                            Err(e) if retry_count < 3 => {
                                warn!(
                                    "⚠️ Realm backup retry {}/3 for realm state {}: {}",
                                    retry_count + 1,
                                    realm_state_req.checkpoint_id,
                                    e
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                            Err(e) => {
                                error!("❌ Realm backup final failure for realm state {}: {}", realm_state_req.checkpoint_id, e);
                            }
                        }
                    }
                }
            }
        }
        warn!("🔚 Realm backup task stopped");
    }
}
