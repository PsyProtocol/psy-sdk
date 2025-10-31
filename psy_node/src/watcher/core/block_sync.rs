use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use plonky2::hash::hash_types::RichField;
use psy_data::{config::store_config::PsyFelt};
use psy_store::{
    node::{coordinator::PsyCoordinatorStoreReaderAsync, realm::PsyRealmStoreReaderAsync},
    store::PsyStore,
};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info};

use crate::watcher::{
    constant::{BLOCK_SYNC_TIMEOUT_SECS, FAILURE_BACKOFF_DURATION, FAILURE_BACKOFF_THRESHOLD},
    error::WatcherError,
    timeout_watcher::WatcherSourceNodeType,
};

pub struct BlockSyncService {
    psy_store: Arc<PsyStore>,
    block_height: Arc<AtomicU64>,
    node_type: WatcherSourceNodeType,
    sync_interval_secs: u64,
}

impl BlockSyncService {
    pub fn new(psy_store: Arc<PsyStore>, block_height: Arc<AtomicU64>, node_type: WatcherSourceNodeType, sync_interval_secs: u64) -> Self {
        Self {
            psy_store,
            block_height,
            node_type,
            sync_interval_secs,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting block height synchronization (interval: {}s)", self.sync_interval_secs);

        let mut ticker = interval(Duration::from_secs(self.sync_interval_secs));
        let mut consecutive_failures = 0;

        loop {
            ticker.tick().await;

            match self.sync_once().await {
                Ok(new_height) => {
                    let old_height = self.block_height.swap(new_height, Ordering::AcqRel);
                    if new_height > old_height {
                        debug!("Block height updated: {} -> {}", old_height, new_height);
                    }
                    consecutive_failures = 0;
                }
                Err(e) => {
                    consecutive_failures += 1;
                    error!("Failed to sync block height (attempt {}): {}", consecutive_failures, e);

                    if consecutive_failures % FAILURE_BACKOFF_THRESHOLD == 0 {
                        debug!("Backing off for {:?} due to failures", FAILURE_BACKOFF_DURATION);
                        tokio::time::sleep(FAILURE_BACKOFF_DURATION).await;
                    }
                }
            }
        }
    }

    async fn sync_once(&self) -> Result<u64> {
        timeout(Duration::from_secs(BLOCK_SYNC_TIMEOUT_SECS), self.fetch_block_height())
            .await
            .map_err(|_| WatcherError::Database("Block height fetch timeout".to_string()))?
    }

    async fn fetch_block_height(&self) -> Result<u64> {
        let fetch_fn = match self.node_type {
            WatcherSourceNodeType::Coordinator => PsyCoordinatorStoreReaderAsync::get_latest_block_state,
            WatcherSourceNodeType::Realm => PsyRealmStoreReaderAsync::get_latest_block_state,
        };

        let block_state = fetch_fn(&self.psy_store).await.map_err(|e| WatcherError::Database(e.to_string()))?;

        Ok(block_state.checkpoint_id)
    }

    pub fn set_block_height(&self, height: u64) {
        self.block_height.store(height, Ordering::Release);
    }

    pub fn get_block_height(&self) -> u64 {
        self.block_height.load(Ordering::Acquire)
    }

    pub async fn fetch_initial_height(&self) -> Result<u64> {
        let initial_height = self.fetch_block_height().await?;
        self.set_block_height(initial_height);
        Ok(initial_height)
    }
}
