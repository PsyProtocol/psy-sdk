use crate::realm::F;
use std::sync::Arc;
use std::time::Duration;
use jsonrpsee::rpc_params;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use serde::{Serialize, Deserialize};
use tracing::{info, error, warn, debug};
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use anyhow::Result;
use qed_data::models::checkpoint::sync_info::CheckpointError;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_core::job::history_queue::{CheckpointHistoryQueueEmitterAsyncImm, CheckpointHistoryQueueConsumerAsyncImm};

const SYNC_INTERVAL: Duration = Duration::from_millis(500);

pub struct CheckpointSyncManager<SR, IQ>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    IQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + Sync + Send + 'static,
{
    store_reader: Arc<SR>,
    sync_queue: Arc<IQ>,
    client: HttpClient,
    current_local_checkpoint_id: u64,
    latest_checkpoint_id: u64,
}

impl<SR, IQ> CheckpointSyncManager<SR, IQ>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    IQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + Sync + Send + 'static,
{
    pub fn new(
        store_reader: Arc<SR>,
        interval_sync_queue: Arc<IQ>,
        coordinator_addr: &str,
    ) -> Result<Self> {
        let client = HttpClientBuilder::default()
            .build(coordinator_addr)
            .map_err(|e| anyhow::anyhow!("Failed to create RPC client to coordinator {}: {:?}", coordinator_addr, e))?;

        Ok(Self {
            store_reader,
            sync_queue: interval_sync_queue,
            client,
            current_local_checkpoint_id: 0,
            latest_checkpoint_id: 0,
        })
    }

    async fn run_sync_loop(mut self) {
        loop {
            tokio::time::sleep(SYNC_INTERVAL).await;

            if !self.should_process_sync_cycle().await {
                continue;
            }

            if !self.update_local_checkpoint().await {
                continue;
            }

            if !self.fetch_coordinator_latest_checkpoint().await {
                continue;
            }

            if self.is_up_to_date() {
                continue;
            }

            info!(
                "Local checkpoint ID: {}, latest checkpoint ID: {}",
                self.current_local_checkpoint_id,
                self.latest_checkpoint_id
            );

            self.sync_missing_checkpoints().await;

            debug!("Finished sync cycle. Waiting for next interval...");
        }
    }

    async fn should_process_sync_cycle(&self) -> bool {
        match self.sync_queue.is_empty().await {
            Ok(is_empty) => {
                if !is_empty {
                    debug!("sync queue is not empty. Waiting for next interval...");
                    return false;
                }
                true
            }
            Err(e) => {
                error!("Failed to check if interval sync queue {:?}", e);
                false
            }
        }
    }

    async fn update_local_checkpoint(&mut self) -> bool {
        debug!("Starting active checkpoint sync cycle...");
        match self.store_reader.get_latest_l2_block_state().await {
            Ok(state) => self.current_local_checkpoint_id = state.checkpoint_id,
            Err(e) => {
                error!("Failed to get latest L2 block state: {:?}", e);
                if let Ok(CheckpointError::NotFound) = e.downcast::<CheckpointError>(){
                    self.current_local_checkpoint_id =  0;
                } else {
                    return false;
                }
            }
        };
        true
    }

    async fn fetch_coordinator_latest_checkpoint(&mut self) -> bool {
        match self.client.request::<LatestCheckpointResponse, _>(
            "qed_get_latest_checkpoint",
            rpc_params![]
        ).await {
            Ok(latest_checkpoint) => {
                self.latest_checkpoint_id = latest_checkpoint.checkpoint_id;
                if self.is_up_to_date() {
                    info!(
                        "Local checkpoint {} is up-to-date with coordinator at checkpoint {}",
                        self.current_local_checkpoint_id,
                        self.latest_checkpoint_id
                    );
                    return false;
                }
                true
            }
            Err(e) => {
                error!("RPC call to coordinator ('qed_get_latest_checkpoint') failed: {:?}", e);
                false
            }
        }
    }

    fn is_up_to_date(&self) -> bool {
        self.current_local_checkpoint_id >= self.latest_checkpoint_id
    }

    fn next_checkpoint_id(&self) -> u64 {
        self.current_local_checkpoint_id + 1
    }

    async fn sync_missing_checkpoints(&mut self) {
        loop {
            let next_checkpoint_id = self.next_checkpoint_id();
            debug!("Attempting to fetch checkpoint {} from coordinator...", next_checkpoint_id);

            let params = rpc_params![next_checkpoint_id];
            match self.client.request::<Option<CheckpointSyncInfo<F>>, _>(
                "qed_get_checkpoint_sync_info",
                params
            ).await {
                Ok(Some(sync_info)) => {
                    if !self.process_checkpoint_sync_info(sync_info, next_checkpoint_id).await {
                        break;
                    }
                }
                Ok(None) => {
                    info!("Realm is up-to-date with coordinator at checkpoint {}", next_checkpoint_id);
                    break;
                }
                Err(e) => {
                    error!("RPC call to coordinator ('qed_get_checkpoint_sync_info') failed: {:?}", e);
                    break;
                }
            }
        }
    }

    async fn process_checkpoint_sync_info(
        &mut self,
        sync_info: CheckpointSyncInfo<F>,
        next_checkpoint_id: u64,
    ) -> bool {
        let latest_checkpoint_id = sync_info.latest_checkpoint_id;
        self.latest_checkpoint_id = latest_checkpoint_id;
        if latest_checkpoint_id <= self.current_local_checkpoint_id {
            if latest_checkpoint_id < self.current_local_checkpoint_id {
                warn!(
                    expected = next_checkpoint_id,
                    received = sync_info.compact.l2_block_state.checkpoint_id,
                    lastest = sync_info.latest_checkpoint_id,
                    "Received out-of-order checkpoint sync info from coordinator. Retrying cycle."
                );
            }
            return false;
        }

        info!(
            latest_checkpoint_id = self.latest_checkpoint_id,
            next_checkpoint_id = next_checkpoint_id,
            source = ?sync_info.source_coordinator_edge_id,
            "Received sync info for next checkpoint from coordinator. Pushing to queue."
        );

        match self.sync_queue.chq_push_imm(sync_info).await {
            Ok(_) => {
                self.current_local_checkpoint_id = next_checkpoint_id;
                if self.current_local_checkpoint_id == latest_checkpoint_id {
                    return false;
                }
                debug!("Successfully pushed sync info. Continuing fetch loop for checkpoint {}.", self.current_local_checkpoint_id);
                true
            }
            Err(e) => {
                error!("Failed to push sync info to internal queue: {:?}", e);
                false
            }
        }
    }
}

pub async fn spawn_active_checkpoint_sync_task<
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    IQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + Sync + Send + 'static,
>(
    store_reader: Arc<SR>,
    interval_sync_queue: Arc<IQ>,
    coordinator_addr: String,
) -> Result<()> {
    info!(coordinator = %coordinator_addr, interval = ?SYNC_INTERVAL, "Spawning active checkpoint sync task");
    let sync_manager = CheckpointSyncManager::new(
        store_reader,
        interval_sync_queue,
        &coordinator_addr,
    )?;

    tokio::spawn(async move {
        sync_manager.run_sync_loop().await;
    });
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatestCheckpointResponse {
    pub checkpoint_id: u64,
}

