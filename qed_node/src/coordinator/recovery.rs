use anyhow::{Context, Result};
use kvq::traits::{KVQBinaryStore, KVQPair};
use qed_store::{
    node::coordinator::QEDCoordinatorStoreReaderAsync,
    store::{journal::JournalStore, QEDStore},
};
use tracing::{error, info, warn};

use super::{args::CoordinatorProcessorArgs, backup::S3BackupClient};

pub struct CoordinatorRecoveryManager {
    store: JournalStore<QEDStore>,
    backup_client: S3BackupClient,
    current_checkpoint_id: u64,
}

impl CoordinatorRecoveryManager {
    pub async fn new(args: &CoordinatorProcessorArgs) -> Result<Self> {
        let backup_client = S3BackupClient::new().await?;
        let qed_store = QEDStore::from_backend(args.backend.to_backend()).await?;
        let store = JournalStore::new(qed_store);
        let current_checkpoint_id = 0;
        Ok(Self {
            store,
            backup_client,
            current_checkpoint_id,
        })
    }

    pub async fn sync_from_s3(&mut self, target_checkpoint: Option<u64>) -> Result<()> {
        info!("🔄 Starting recovery from S3...");

        // Get recovery info from S3
        let recovery_info = self.backup_client.fetch_recovery_info().await?;
        info!(
            "📋 Recovery info: latest checkpoint {}, available: {:?}",
            recovery_info.latest_checkpoint, recovery_info.checkpoints_available
        );

        // Determine which checkpoint to recover to
        let target = target_checkpoint.unwrap_or(recovery_info.latest_checkpoint);
        let start_checkpoint = self.current_checkpoint_id + 1;

        info!("📊 Current local checkpoint: {}", self.current_checkpoint_id);
        info!("⚡ Recovering from checkpoint {} to {}", start_checkpoint, target);

        // Recover checkpoints incrementally
        for checkpoint_id in start_checkpoint..=target {
            if !recovery_info.checkpoints_available.contains(&checkpoint_id) {
                warn!("⚠️ Checkpoint {} not available in S3, skipping", checkpoint_id);
                continue;
            }

            match self.recover_checkpoint(checkpoint_id).await {
                Ok(()) => {
                    info!("✅ Successfully recovered checkpoint {}", checkpoint_id);
                    self.current_checkpoint_id = checkpoint_id; // Update current checkpoint
                }
                Err(e) => {
                    error!("❌ Failed to recover checkpoint {}: {}", checkpoint_id, e);
                    return Err(e);
                }
            }
        }

        info!("🎉 Recovery completed! Final checkpoint: {}", target);
        Ok(())
    }

    async fn recover_checkpoint(&mut self, checkpoint_id: u64) -> Result<()> {
        // Fetch backup from S3
        let backup = self
            .backup_client
            .fetch_checkpoint_backup(checkpoint_id)
            .await
            .context(format!("Failed to fetch backup for checkpoint {}", checkpoint_id))?;

        info!(
            "📝 Applying {} journal changes and {} removed keys for checkpoint {}",
            backup.pair_to_set.len(),
            backup.removed_keys.len(),
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

        info!("💾 Committed checkpoint {} with {} changes", checkpoint_id, backup.pair_to_set.len());

        Ok(())
    }

    pub async fn verify_recovery(&self, expected_checkpoint: Option<u64>) -> Result<()> {
        if let Some(expected) = expected_checkpoint {
            if self.current_checkpoint_id != expected {
                return Err(anyhow::anyhow!(
                    "Recovery verification failed: expected checkpoint {}, got {}",
                    expected,
                    self.current_checkpoint_id
                ));
            }
        }

        info!("✅ Recovery verification passed: checkpoint {}", self.current_checkpoint_id);
        Ok(())
    }

    pub async fn list_available_backups(&self) -> Result<Vec<u64>> {
        self.backup_client.list_available_checkpoints().await
    }
}

// CLI command implementations
pub async fn run_sync_command(args: CoordinatorProcessorArgs, target_checkpoint: Option<u64>) -> Result<()> {
    let mut recovery_manager = CoordinatorRecoveryManager::new(&args).await?;

    recovery_manager.sync_from_s3(target_checkpoint).await?;
    recovery_manager.verify_recovery(target_checkpoint).await?;

    Ok(())
}

pub async fn run_list_backups_command() -> Result<()> {
    let backup_client = S3BackupClient::new().await?;
    let checkpoints = backup_client.list_available_checkpoints().await?;

    if checkpoints.is_empty() {
        info!("No backups found in S3");
    } else {
        info!("Available backup checkpoints:");
        for checkpoint in checkpoints {
            info!("  - {}", checkpoint);
        }
    }

    Ok(())
}
