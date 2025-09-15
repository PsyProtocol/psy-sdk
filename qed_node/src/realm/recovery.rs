use anyhow::{Context, Result};
use kvq::traits::{KVQBinaryStore, KVQPair};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_crypto::common::user_id::get_user_id_from_registration_id;
use qed_data::{
    config::store_config::{CheckpointSyncInfoTableStore, QCheckpointSyncInfoCompact},
    models::checkpoint::sync_info::QEDCheckpointSyncInfoModelReaderCore,
};
use qed_store::{
    queue::{new_redis_async_pool, ProofStoreRedisAsync, QPendingUserStoreAsyncImm},
    store::{journal::JournalStore, QEDStore},
};
use tracing::{error, info, warn};

use super::backup::RealmS3BackupClient;

pub struct RealmRecoveryManager {
    realm_id: u32,
    qed_store: QEDStore,
    store: JournalStore<QEDStore>,
    backup_client: RealmS3BackupClient,
    sync_queue: std::sync::Arc<ProofStoreRedisAsync>,
    current_checkpoint_id: u64,
}

impl RealmRecoveryManager {
    pub async fn new(
        realm_id: u32,
        backend: qed_store::store::backend::Backend,
        bucket: String,
        redis_uri: String,
        queue_biz_key: String,
        redis_pool_size: Option<usize>,
    ) -> Result<Self> {
        let backup_client = RealmS3BackupClient::new(realm_id, bucket).await?;
        let qed_store = QEDStore::from_backend(backend).await?;
        let store = JournalStore::new(qed_store.clone());

        // Initialize sync_queue using redis configuration
        let pool = new_redis_async_pool(&redis_uri, redis_pool_size.unwrap_or(10)).await?;
        let sync_queue = std::sync::Arc::new(ProofStoreRedisAsync::new(pool, queue_biz_key).await?);

        Ok(Self {
            realm_id,
            qed_store,
            store,
            backup_client,
            sync_queue,
            current_checkpoint_id: 0,
        })
    }

    pub async fn sync_from_s3(&mut self, target_checkpoint: Option<u64>) -> Result<()> {
        info!("🔄 Starting realm {} recovery from S3...", self.realm_id);

        // Get recovery info from S3
        let recovery_info = self.backup_client.fetch_recovery_info().await?;
        info!(
            "📋 Realm {} recovery info: latest checkpoint {}, available: {:?}",
            self.realm_id, recovery_info.latest_checkpoint, recovery_info.checkpoints_available
        );

        // Determine which checkpoint to recover to
        let target = target_checkpoint.unwrap_or(recovery_info.latest_checkpoint);
        let start_checkpoint = self.current_checkpoint_id + 1;

        info!("📊 Current local checkpoint: {}", self.current_checkpoint_id);
        info!("⚡ Recovering realm {} from checkpoint {} to {}", self.realm_id, start_checkpoint, target);

        // Recover checkpoints incrementally
        for checkpoint_id in start_checkpoint..=target {
            if !recovery_info.checkpoints_available.contains(&checkpoint_id) {
                warn!(
                    "⚠️ Checkpoint {} not available in S3 for realm {}, skipping",
                    checkpoint_id, self.realm_id
                );
                continue;
            }

            match self.recover_checkpoint(checkpoint_id).await {
                Ok(()) => {
                    info!("✅ Successfully recovered realm {} checkpoint {}", self.realm_id, checkpoint_id);
                    self.current_checkpoint_id = checkpoint_id; // Update current checkpoint
                }
                Err(e) => {
                    error!("❌ Failed to recover realm {} checkpoint {}: {}", self.realm_id, checkpoint_id, e);
                    return Err(e);
                }
            }
        }

        info!("🎉 Realm {} recovery completed! Final checkpoint: {}", self.realm_id, target);
        Ok(())
    }

    async fn recover_checkpoint(&mut self, checkpoint_id: u64) -> Result<()> {
        info!("🔧 Starting realm {} checkpoint {} recovery process", self.realm_id, checkpoint_id);

        // Step 1: Connect to realm's sync_queue (Redis)
        info!("1️⃣ Connected to realm {} sync_queue", self.realm_id);

        // Step 2: Execute peek_with_position(32, checkpoint_id) to simulate build_block
        // method
        info!("2️⃣ Simulating build_block pending users processing...");
        let (pending_users, _consumption_state) = self
            .sync_queue
            .peek_with_position::<GoldilocksField>(32, checkpoint_id)
            .await
            .context("Failed to peek pending users from sync_queue")?;

        info!("📋 Found {} pending users for checkpoint {}", pending_users.len(), checkpoint_id);

        // Step 3: Recover checkpoint data (similar to coordinator processor backup)
        info!("3️⃣ Recovering checkpoint data from S3...");
        let backup = self
            .backup_client
            .fetch_checkpoint_backup(checkpoint_id)
            .await
            .context(format!("Failed to fetch backup for realm {} checkpoint {}", self.realm_id, checkpoint_id))?;

        info!(
            "📝 Applying {} journal changes and {} removed keys for realm {} checkpoint {}",
            backup.pair_to_set.len(),
            backup.removed_keys.len(),
            self.realm_id,
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
        self.store.set_and_delete_many(&pair_to_set_ref, &removed_keys)?;

        // Step 4: Get QCheckpointSyncInfoCompact from store for this checkpoint,
        // check if there are any realm users
        info!("4️⃣ Checking for realm users in checkpoint sync info...");

        if let Ok(sync_info) = self.get_checkpoint_sync_info(checkpoint_id).await {
            // Reference handle_checkpoint_sync method implementation
            let dmps = sync_info.get_registered_user_merkle_proofs::<qed_data::config::store_config::QEDHasher>();

            // Filter users that belong to this realm (consistent with
            // handle_checkpoint_sync logic)
            let realm_users: Vec<_> = dmps
                .into_iter()
                .filter(|x| {
                    let real_id = get_user_id_from_registration_id(x.index);
                    self.includes_user_id(real_id)
                })
                .collect();

            if !realm_users.is_empty() {
                info!("📤 Pushing {} realm users to sync_queue", realm_users.len());
                self.sync_queue.push_pending_users(&realm_users).await?;
            } else {
                info!("ℹ️ No realm users found for realm {} in checkpoint {}", self.realm_id, checkpoint_id);
            }
        } else {
            info!(
                "ℹ️ No checkpoint sync info found for checkpoint {} (this is normal for some checkpoints)",
                checkpoint_id
            );
        }

        // Step 5: Execute commit_offset operation (update pending_user_list queue)
        info!("5️⃣ Committing offset...");
        if let Some(state) = self.sync_queue.get_last_peek_offset().await? {
            self.sync_queue.commit_offset(&state).await?;
            info!("✅ Committed offset for realm {} checkpoint {}", self.realm_id, checkpoint_id);
        }

        info!(
            "💾 Committed realm {} checkpoint {} with {} changes",
            self.realm_id,
            checkpoint_id,
            backup.pair_to_set.len()
        );

        Ok(())
    }

    // Helper method to check if a user ID belongs to this realm
    // This logic should match RealmConfig::includes_user_id
    fn includes_user_id(&self, user_id: u64) -> bool {
        let users_per_realm = 1usize << (qed_core::config::network_constants::REALM_USER_TREE_HEIGHT as usize);
        let r64 = self.realm_id as u64;
        user_id >= r64 * (users_per_realm as u64) && user_id < (r64 + 1) * (users_per_realm as u64)
    }

    // Helper method to get checkpoint sync info from store
    async fn get_checkpoint_sync_info(&self, checkpoint_id: u64) -> Result<QCheckpointSyncInfoCompact> {
        CheckpointSyncInfoTableStore::<QEDStore>::get_checkpoint_sync_info_compact_or_latest(&self.qed_store, checkpoint_id)
    }

    pub async fn verify_recovery(&self, expected_checkpoint: Option<u64>) -> Result<()> {
        if let Some(expected) = expected_checkpoint {
            if self.current_checkpoint_id != expected {
                return Err(anyhow::anyhow!(
                    "Realm {} recovery verification failed: expected checkpoint {}, got {}",
                    self.realm_id,
                    expected,
                    self.current_checkpoint_id
                ));
            }
        }

        info!(
            "✅ Realm {} recovery verification passed: checkpoint {}",
            self.realm_id, self.current_checkpoint_id
        );
        Ok(())
    }

    pub async fn list_available_backups(&self) -> Result<Vec<u64>> {
        self.backup_client.list_available_checkpoints().await
    }
}

// CLI command implementations
pub async fn run_realm_sync_command(
    realm_id: u32,
    target_checkpoint: Option<u64>,
    aws_bucket: String,
    backend_config: qed_store::store::backend::BackendConfig,
    redis_uri: String,
    queue_biz_key: String,
    redis_pool_size: Option<usize>,
) -> Result<()> {
    let backend = backend_config.to_backend();
    let mut recovery_manager = RealmRecoveryManager::new(realm_id, backend, aws_bucket, redis_uri, queue_biz_key, redis_pool_size).await?;
    recovery_manager.sync_from_s3(target_checkpoint).await?;
    recovery_manager.verify_recovery(target_checkpoint).await?;
    Ok(())
}
