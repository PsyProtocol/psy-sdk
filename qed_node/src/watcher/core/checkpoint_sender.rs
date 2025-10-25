// checkpoint_sender.rs - Separate checkpoint leaf sending logic
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use qed_core::config::network_constants::SLOT_SIZE;
use qed_data::config::store_config::QEDFelt;
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;
use qed_store::store::QEDStore;
use crate::watcher::ApiClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::common::retry::{retry_with_backoff, RetryConfig};
use crate::watcher::constant::BLOCK_METADATA_FINALIZATION_DELAY;
use crate::watcher::error::WatcherError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointLeafWithId {
    pub checkpoint_id: u64,
    pub checkpoint_leaf: QEDCheckpointLeaf<QEDFelt>,
}

pub struct CheckpointSenderService {
    qed_store: Arc<QEDStore>,
    api_client: Arc<ApiClient>,
    latest_height: Arc<AtomicU64>,
    local_height: Arc<AtomicU64>,
    wait_duration: Duration,
}

impl CheckpointSenderService {
    pub fn new(
        qed_store: Arc<QEDStore>,
        api_client: Arc<ApiClient>,
        latest_height: Arc<AtomicU64>,
    ) -> Self {
        Self {
            qed_store,
            api_client,
            latest_height,
            local_height: Arc::new(AtomicU64::new(0)),
            wait_duration: Duration::from_millis(BLOCK_METADATA_FINALIZATION_DELAY * SLOT_SIZE),
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting checkpoint leaf sender, wait secs : {}", self.wait_duration.as_secs());

        let mut interval = tokio::time::interval(self.wait_duration);
        loop {
            interval.tick().await;
            if let Err(e) = self.process_checkpoints().await {
                error!("❌ Failed to process checkpoints: {e}");
            }
        }
    }

    async fn process_checkpoints(&self) -> Result<()> {
        let latest_height = self.get_latest_height().await;
        let finalized_height = latest_height.saturating_sub(BLOCK_METADATA_FINALIZATION_DELAY);
        let local_height = self.get_local_height().await;
        debug!("🆕 CheckpointSenderService: latest_height={}, finalized_height={}, local_height={}",
            latest_height, finalized_height, local_height);

        if finalized_height <= local_height {
            debug!(
                "CheckpointSenderService: No new finalized checkpoints (finalized={}, local={}), skipping.",
                finalized_height, local_height
            );
            return Ok(());
        }

        let checkpoint_leaves = self.fetch_checkpoint_range(local_height, finalized_height).await?;

        debug!("checkpoint leaves fetched: {}", checkpoint_leaves.len());

        if checkpoint_leaves.is_empty() {
            warn!(
                "⚠️ No checkpoint leaves fetched between {} and {}. Skipping send.",
                local_height, finalized_height
            );
            return Ok(());
        }

        debug!("🚀 Sending {} checkpoint leaves to API client...", checkpoint_leaves.len());
        self.api_client.send_checkpoint_leaves(checkpoint_leaves).await
            .map_err(|e| WatcherError::ApiClient(e.to_string()))?;
        debug!("✅ Successfully sent checkpoint leaves to API client.");

        self.local_height.store(finalized_height, Ordering::Relaxed);

        info!(
            "Successfully sent checkpoints {} to {}",
            local_height, finalized_height
        );

        Ok(())
    }

    async fn get_latest_height(&self) -> u64 {
        self.latest_height.load(Ordering::Relaxed)
    }
    async fn get_local_height(&self) -> u64 {
        self.local_height.load(Ordering::Relaxed)
    }
    async fn set_local_height(&self, new_height: u64) {
        self.local_height.store(new_height, Ordering::Relaxed);
    }

    async fn fetch_checkpoint_range(
        &self,
        start: u64,
        end: u64,
    ) -> Result<Vec<CheckpointLeafWithId>> {
        let mut checkpoint_leaves = Vec::new();

        for checkpoint_id in start..end {
            match self.fetch_single_checkpoint(checkpoint_id).await {
                Ok(leaf) => {
                    checkpoint_leaves.push(CheckpointLeafWithId {
                        checkpoint_id,
                        checkpoint_leaf: leaf,
                    });
                }
                Err(e) => {
                    error!(
                        "Failed to fetch checkpoint {} after retries: {}. Aborting this batch.",
                        checkpoint_id, e
                    );
                    return Err(e);
                }
            }
        }

        Ok(checkpoint_leaves)
    }

    async fn fetch_single_checkpoint(&self, checkpoint_id: u64) -> Result<QEDCheckpointLeaf<QEDFelt>> {
        let config = RetryConfig::default();

        retry_with_backoff(
            &config,
            &format!("fetch checkpoint {}", checkpoint_id),
            || async {
                QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data(
                    &self.qed_store,
                    checkpoint_id,
                )
                .await
                .map_err(|e| WatcherError::Database(e.to_string()))
            },
        )
            .await
    }
}