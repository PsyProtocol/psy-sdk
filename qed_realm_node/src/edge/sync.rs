use crate::{rpc::CheckpointSyncInfo, SyncCheckpointQueue, F};
use std::sync::Arc;
use std::time::Duration;
use jsonrpsee::rpc_params;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use serde::{Serialize,Deserialize};
use tracing::{info, error, warn, debug};
use qed_store::node::realm::QEDRealmStoreReaderAsync;


// Define the sync interval
const SYNC_INTERVAL: Duration = Duration::from_millis(500); // Example: sync every 500 milliseconds

pub async fn spawn_active_checkpoint_sync_task<
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    IQ: SyncCheckpointQueue + Sync + Send + 'static,
>(
    store_reader: Arc<SR>,
    interval_sync_queue: Arc<IQ>,
    coordinator_addr: String,
) -> anyhow::Result<()> {
    info!(coordinator = %coordinator_addr, interval = ?SYNC_INTERVAL, "Spawning active checkpoint sync task");
    // Build the RPC client for each attempt or reuse if possible and safe
    let client = match HttpClientBuilder::default().build(&coordinator_addr) {
        Ok(c) => c,
        Err(e) => {
            anyhow::bail!("Failed to create RPC client to coordinator {}: {:?}", coordinator_addr, e);
        }
    };

    let mut current_local_checkpoint_id = 0;
    let mut latest_checkpoint_id = 0;
    tokio::spawn(async move {
        loop {
            // Wait for the defined interval before starting the next cycle
            tokio::time::sleep(SYNC_INTERVAL).await;
            match interval_sync_queue.is_empty().await {
                Ok(is_empty) => {
                    if !is_empty {
                        debug!("sync queue is not empty. Waiting for next interval...");
                        continue;
                    }
                }
                Err(e) => {
                    error!("Failed to check if interval sync queue {:?}", e);
                }
            }
            
            debug!("Starting active checkpoint sync cycle...");
            current_local_checkpoint_id = store_reader.get_latest_l2_block_state().await.unwrap_or_default().checkpoint_id;

            // Call the coordinator's new RPC method
            match client.request::<Option<LatestCheckpointResponse>, _>("qed_get_latest_checkpoint", rpc_params![]).await {
                Ok(Some(latest_checkpoint)) => {
                    latest_checkpoint_id = latest_checkpoint.checkpoint_id;
                    if current_local_checkpoint_id >= latest_checkpoint.checkpoint_id {
                        info!("Local checkpoint {} is up-to-date with coordinator at checkpoint {}", current_local_checkpoint_id, latest_checkpoint.checkpoint_id);
                        continue
                    }
                }
                Ok(None) => {
                    // Coordinator indicates no newer checkpoint available
                    info!("Local checkpoint {} is up-to-date with coordinator at checkpoint", current_local_checkpoint_id);
                    // Break the inner loop, we are caught up for now.
                    continue;
                }
                Err(e) => {
                    error!("RPC call to coordinator ('qed_get_latest_checkpoint') failed: {:?}", e);
                    continue;
                }
            }

            info!("Local checkpoint ID: {}, latest checkpoint ID: {}", current_local_checkpoint_id, latest_checkpoint_id);
            // Inner loop to fetch potentially multiple missing checkpoints
            loop {
                let next_checkpoint_id= current_local_checkpoint_id +1;
                debug!("Attempting to fetch checkpoint {} from coordinator...", next_checkpoint_id);

                // Call the coordinator's new RPC method
                let params = rpc_params![next_checkpoint_id];
                match client.request::<Option<CheckpointSyncInfo>, _>("qed_get_checkpoint_sync_info", params).await {
                    Ok(Some(sync_info)) => {
                        // Check if the received checkpoint is the one we expected
                        let lastest_checkpoint_id = sync_info.latest_checkpoint_id;
                        if lastest_checkpoint_id <= current_local_checkpoint_id {
                            if lastest_checkpoint_id< current_local_checkpoint_id {
                                warn!(
                                    expected = next_checkpoint_id,
                                    received = sync_info.compact.l2_block_state.checkpoint_id,
                                    lastest = sync_info.latest_checkpoint_id,
                                    "Received out-of-order checkpoint sync info from coordinator. Retrying cycle."
                                );
                            }
                            // Break inner loop and retry the whole cycle in the next interval
                            break;
                        }

                        info!(
                            latest_checkpoint_id = latest_checkpoint_id,
                            next_checkpoint_id = next_checkpoint_id,
                            source = ?sync_info.source_coordinator_edge_id,
                            "Received sync info for next checkpoint from coordinator. Pushing to queue."
                        );

                        // Push the compact info to the internal queue for processing
                        match interval_sync_queue.produce_checkpoint_async_info(sync_info).await {
                            Ok(_) => {
                                // Successfully processed, update local checkpoint ID for the *next* fetch in this inner loop
                                current_local_checkpoint_id = next_checkpoint_id;
                                if current_local_checkpoint_id == lastest_checkpoint_id {
                                    break;
                                }
                                debug!("Successfully pushed sync info. Continuing fetch loop for checkpoint {}.", current_local_checkpoint_id);
                                // Continue the inner loop immediately to fetch the next one
                            }
                            Err(e) => {
                                error!("Failed to push sync info to internal queue: {:?}", e);
                                // Break inner loop, wait for next outer interval cycle
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        // Coordinator indicates no newer checkpoint available
                        info!("Realm is up-to-date with coordinator at checkpoint {}", next_checkpoint_id);
                        // Break the inner loop, we are caught up for now.
                        break;
                    }
                    Err(e) => {
                        error!("RPC call to coordinator ('qed_get_checkpoint_sync_info') failed: {:?}", e);
                        // Break inner loop, wait for next outer interval cycle
                        break;
                    }
                }
            } // End of inner loop (fetching until caught up or error)
            debug!("Finished sync cycle. Waiting for next interval...");
        } // End of outer loop
    }); // End of tokio::spawn

    Ok(())
}

#[derive(Debug, Clone, Deserialize,Serialize)]
pub struct LatestCheckpointResponse {
    pub checkpoint_id: u64,
}