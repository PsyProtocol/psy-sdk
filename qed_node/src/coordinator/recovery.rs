use anyhow::{Context, Result};
use qed_data::{config::store_config::QEDFelt, qdata::checkpoint::QEDL2BlockState};
use qed_store::store::{journal::JournalStore, QEDStore};
use super::backup::{S3BackupClient, CheckpointBackup, RecoveryInfo};
use super::args::CoordinatorProcessorArgs;
use tracing::{info, warn, error};

type F = QEDFelt;

pub struct CoordinatorRecoveryManager {
    store: JournalStore<QEDStore>,
    backup_client: S3BackupClient,
}

impl CoordinatorRecoveryManager {
    pub async fn new(args: &CoordinatorProcessorArgs) -> Result<Self> {
        let backup_client = S3BackupClient::new().await?;
        let qed_store = QEDStore::from_backend(args.backend.to_backend()).await?;
        let store = JournalStore::new(qed_store);

        Ok(Self {
            store,
            backup_client,
        })
    }

    pub async fn sync_from_s3(&mut self, target_checkpoint: Option<u64>) -> Result<()> {
        info!("🔄 Starting recovery from S3...");

        // Get recovery info from S3
        let recovery_info = self.backup_client.fetch_recovery_info().await?;
        info!("📋 Recovery info: latest checkpoint {}, available: {:?}", 
              recovery_info.latest_checkpoint, recovery_info.checkpoints_available);

        // Determine which checkpoint to recover to
        let target = target_checkpoint.unwrap_or(recovery_info.latest_checkpoint);
        
        // Get current local state
        let current_l2_state = self.store.get_latest_l2_block_state().await
            .unwrap_or_else(|_| {
                warn!("No local L2 state found, starting from genesis");
                create_genesis_l2_state()
            });

        let start_checkpoint = current_l2_state.checkpoint_id + 1;
        
        info!("📊 Current local checkpoint: {}", current_l2_state.checkpoint_id);
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
        let backup = self.backup_client.fetch_checkpoint_backup(checkpoint_id).await
            .context(format!("Failed to fetch backup for checkpoint {}", checkpoint_id))?;

        // Apply journal changes
        if !backup.journal_changes.is_empty() {
            info!("📝 Applying {} journal changes for checkpoint {}", 
                  backup.journal_changes.len(), checkpoint_id);
            
            for (key, value) in &backup.journal_changes {
                self.store.set_ref(key, value)?;
            }
        }

        // Commit changes
        self.store.commit(checkpoint_id)?;

        info!("💾 Committed checkpoint {} with {} changes", 
              checkpoint_id, backup.journal_changes.len());

        Ok(())
    }

    pub async fn verify_recovery(&self, expected_checkpoint: Option<u64>) -> Result<()> {
        let current_state = self.store.get_latest_l2_block_state().await?;
        
        if let Some(expected) = expected_checkpoint {
            if current_state.checkpoint_id != expected {
                return Err(anyhow::anyhow!(
                    "Recovery verification failed: expected checkpoint {}, got {}",
                    expected, current_state.checkpoint_id
                ));
            }
        }

        info!("✅ Recovery verification passed: checkpoint {}", current_state.checkpoint_id);
        Ok(())
    }

    pub async fn list_available_backups(&self) -> Result<Vec<u64>> {
        self.backup_client.list_available_checkpoints().await
    }
}

fn create_genesis_l2_state() -> QEDL2BlockState<F> {
    QEDL2BlockState {
        checkpoint_id: 0,
        user_tree_root: qed_core::data::qhashout::QHashOut::ZERO,
        contract_tree_root: qed_core::data::qhashout::QHashOut::ZERO,
        user_registration_tree_root: qed_core::data::qhashout::QHashOut::ZERO,
        next_user_id: 0,
        next_contract_id: 0,
        pow_rewards_checkpoint_id: 0,
        last_claimed_pow_rewards_checkpoint_id: 0,
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