use anyhow::{Context, Result};
use aws_sdk_s3::config::retry::ShouldAttempt::No;
use kvq::traits::{KVQBinaryStore, KVQPair};
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_config::GenesisConfigGoldilocks as GenesisConfig;
use psy_store::{
    node::coordinator::PsyCoordinatorStoreReaderAsync,
    store,
    store::{
        journal::{Journal, JournalStore},
        PsyStore,
    },
};
use tracing::{error, info, warn};

use super::backup::CoordinatorS3BackupClient;
use crate::coordinator::CoordinatorProcessNode;

pub struct CoordinatorRecoveryManager {
    store: JournalStore<PsyStore>,
    backup_client: CoordinatorS3BackupClient,
    current_checkpoint_id: u64,
    config_path: String,
}

impl CoordinatorRecoveryManager {
    pub async fn new(backend: psy_store::store::backend::Backend, bucket: String, config_path: String) -> Result<Self> {
        let backup_client = CoordinatorS3BackupClient::new(bucket).await?;
        info!("Initialized S3BackupClient for recovery");
        let psy_store = store::from_backend(backend).await?;
        info!("Initialized PsyStore for recovery");
        let store = JournalStore::new(psy_store);
        info!("Initialized JournalStore for recovery");
        let current_checkpoint_id = 0;
        Ok(Self {
            store,
            backup_client,
            current_checkpoint_id,
            config_path,
        })
    }

    pub async fn sync_from_s3(&mut self, target_checkpoint: Option<u64>) -> Result<()> {
        info!("🔄 Starting coordinator recovery from S3...");
        let config = psy_config::PsyConfigGoldilocks::from_file("config.json")?;
        let network = config.get_current_network()?;
        let genesis_config = if let Some(genesis) = &network.genesis {
            let genesis_json = serde_json::to_string(genesis)?;
            Some(GenesisConfig::from_json(&genesis_json)?)
        } else {
            None
        };

        // Get recovery info from S3
        let recovery_info = self.backup_client.fetch_recovery_info().await?;
        info!(
            "📋 Recovery info: latest checkpoint {}, available: {:?}",
            recovery_info.latest_checkpoint, recovery_info.checkpoints_available
        );

        // Determine which checkpoint to recover to
        let target_checkpoint_id = target_checkpoint.unwrap_or(recovery_info.latest_checkpoint);
        self.current_checkpoint_id = CoordinatorProcessNode::initialize_store(&self.store, genesis_config).await?;
        if self.current_checkpoint_id == 0 {
            info!("Initialized store to genesis state, commit checkpoint 0");
            self.store.commit(None).await?;
        }

        let start_checkpoint = self.current_checkpoint_id + 1;

        info!("📊 Current local checkpoint: {}", self.current_checkpoint_id);
        info!("⚡ Recovering from checkpoint {} to {}", start_checkpoint, target_checkpoint_id);

        // Recover checkpoints incrementally
        for checkpoint_id in start_checkpoint..=target_checkpoint_id {
            if !recovery_info.checkpoints_available.contains(&checkpoint_id) {
                warn!("⚠️ Checkpoint {} not available in S3, skipping", checkpoint_id);
                continue;
            }

            match self.recover_checkpoint(checkpoint_id).await {
                Ok(()) => {
                    info!("✅ Successfully recovered checkpoint {}", checkpoint_id);
                    self.store.commit(None).await?;
                    self.current_checkpoint_id = checkpoint_id; // Update current checkpoint
                }
                Err(e) => {
                    error!("❌ Failed to recover checkpoint {}: {}", checkpoint_id, e);
                    return Err(e);
                }
            }
        }

        info!("🎉 Recovery completed! Final checkpoint: {}", target_checkpoint_id);
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
        if let Some(expected_checkpoint) = expected_checkpoint {
            let latest_state = self.store.get_latest_block_state().await?;
            if latest_state.checkpoint_id == expected_checkpoint {
                info!("✅ Successfully recovered to checkpoint {}", latest_state.checkpoint_id);
            } else {
                error!(
                    "❌ Recovery verification failed: expected checkpoint {}, latest checkpoint: {}",
                    expected_checkpoint, latest_state.checkpoint_id
                );
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
pub async fn run_sync_command(
    target_checkpoint: Option<u64>,
    aws_bucket: String,
    backend_config: psy_store::store::backend::BackendConfig,
    config_path: String,
) -> Result<()> {
    info!("{:#?}", backend_config);
    info!("Using AWS S3 bucket: {}", aws_bucket);
    info!("Target checkpoint: {:?}", target_checkpoint);
    let backend = backend_config.to_backend();
    info!("Initialized backend: {:?}", backend);
    let mut recovery_manager = CoordinatorRecoveryManager::new(backend, aws_bucket, config_path).await?;
    recovery_manager.sync_from_s3(target_checkpoint).await?;
    recovery_manager.verify_recovery(target_checkpoint).await?;
    Ok(())
}
