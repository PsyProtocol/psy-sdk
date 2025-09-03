use std::env;
use crate::realm::F;
use std::sync::Arc;
use std::time::Duration;
use jsonrpsee::rpc_params;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use serde::{Serialize, Deserialize};
use tracing::{info, error, warn, debug, trace};
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use anyhow::{anyhow, Result};
use http::{HeaderMap, HeaderValue};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::config::network_constants::REALM_PROOF_SYNC_CHANNEL;
use qed_core::job::drain_queue::CheckpointDrainQueueEmitterAsyncImm;
use qed_data::models::checkpoint::sync_info::CheckpointError;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_core::job::history_queue::{CheckpointHistoryQueueEmitterAsyncImm, CheckpointHistoryQueueConsumerAsyncImm};
use qed_core::job::id::{ProvingJobDataId, QProvingJobDataID};
use qed_core::job::traits::QProofStoreAsyncImm;
use qed_data::config::store_config::QEDFelt;
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput};
use qed_rollup_utils::generate_jwt_token;
use qed_store::queue::ProofStoreRedisAsync;
use crate::common::retry::{RetryConfig, Retryable};
use crate::realm::state::edge::RealmEdgeContext;

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
    is_genesis: bool,
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
            is_genesis: false,
        })
    }

    async fn run_sync_loop(mut self) {
        loop {
            tokio::time::sleep(SYNC_INTERVAL).await;

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

    async fn update_local_checkpoint(&mut self) -> bool {
        trace!("Starting active checkpoint sync cycle...");
        match self.store_reader.get_latest_l2_block_state().await {
            Ok(state) => {
                self.current_local_checkpoint_id = state.checkpoint_id;
                self.is_genesis = true;
            }
            Err(e) => {
                error!("Failed to get latest L2 block state: {:?}", e);
                if let Ok(CheckpointError::NotFound) = e.downcast::<CheckpointError>(){
                    self.current_local_checkpoint_id = 0;
                    self.is_genesis = false;
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
                    trace!(
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
        && self.is_genesis
    }

    fn next_checkpoint_id(&self) -> u64 {
        if !self.is_genesis {
            0
        } else {
            self.current_local_checkpoint_id + 1
        }
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
                    trace!("Realm is up-to-date with coordinator at checkpoint {}", next_checkpoint_id);
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
        if self.is_up_to_date() {
            warn!(
                expected = next_checkpoint_id,
                received = sync_info.compact.l2_block_state.checkpoint_id,
                lastest = sync_info.latest_checkpoint_id,
                "Received out-of-order checkpoint sync info from coordinator. Retrying cycle."
            );
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

pub async fn spawn_realm_job_update_task<
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: QProofStoreAsyncImm + Sync + Send + 'static,
>(
    proof_store: Arc<ProofStoreRedisAsync>,
    realm_id: u64,
    coordinator_addr: String,
    ctx: Arc<RealmEdgeContext<SR, DQ, PS>>,
    retry_config: Option<RetryConfig>,
) -> Result<()> {
    info!("realm job listener spawned");

    // Create RealmProofSender instance once
    let proof_sender = Arc::new(RealmProofSender::new(realm_id, coordinator_addr, retry_config)?);
    let mut last_checkpoint = match ctx.get_checkpoint_id_async().await {
        Ok(checkpoint) => {
            let next_checkpoint = checkpoint + 1;
            info!("Starting realm job update task from checkpoint: {} (latest local: {})", next_checkpoint, checkpoint);
            next_checkpoint
        },
        Err(e) => {
            warn!("Failed to get latest local checkpoint, starting from 0: {}", e);
            0u64
        }
    };
    tokio::spawn(async move {
        loop {
            // Listen for new proof job IDs from the history queue
            match proof_store
                .wait_for_next_item_imm::<ProvingJobDataId>(
                    REALM_PROOF_SYNC_CHANNEL,
                    last_checkpoint,
                )
                .await
            {
                Ok(job_id) => {
                    info!(?job_id, "Received proof from realm processor");
                    last_checkpoint = job_id.checkpoint_id + 1;

                    // Use the RealmProofSender instance
                    if let Err(err) = proof_sender.send_proof(ctx.proof_store.clone(), job_id).await {
                        error!("Failed to send realm proof: {:?}", err);
                    }
                }
                Err(err) => {
                    error!("Error getting job_id from history queue: {:?}", err);
                    // Avoid busy waiting on error
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });
    Ok(())
}

/// Handles sending realm proofs to coordinator with optimized HTTP client reuse and retry mechanisms
pub struct RealmProofSender {
    realm_id: u64,
    http_client: HttpClient,
    retry_config: RetryConfig,
}

impl RealmProofSender {
    /// Create a new RealmProofSender instance
    pub fn new(realm_id: u64, coordinator_addr: String, retry_config: Option<RetryConfig>) -> Result<Self> {
        let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");
        let jwt_token = generate_jwt_token(&secret, realm_id)?;
        let bearer_token_value = format!("Bearer {}", jwt_token);
        let header_value = HeaderValue::from_str(&bearer_token_value)?;

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", header_value);

        let http_client = HttpClientBuilder::default()
            .set_headers(headers)
            .build(&coordinator_addr)?;

        Ok(Self {
            realm_id,
            http_client,
            retry_config: retry_config.unwrap_or_default(),
        })
    }

    /// Send realm proof to coordinator with unified retry mechanism
    pub async fn send_proof<PS: QProofStoreAsyncImm>(
        &self,
        proof_store: Arc<PS>,
        job_info: ProvingJobDataId,
    ) -> Result<()> {
        info!(?job_info.job_id, "send_realm_proof start");
        // Get bytes with retry
        // let bytes = self.get_bytes_with_retry(proof_store.clone(), job_info.job_id).await?;
        let bytes = proof_store.get_bytes_by_id(job_info.job_id).await?;
        // Deserialize realm result
        let realm_result: GUTARealmCheckpointResult<QEDFelt> = bincode::deserialize(&bytes)?;
        // Get proof with retry
        let proof =  proof_store.get_proof_by_id(realm_result.proof_id.get_output_id()).await?;
        // let proof = self.get_proof_with_retry(proof_store, realm_result.proof_id.get_output_id()).await?;
        let input = SubmitGUTARealmResultAPINoProofInput::<QEDFelt> {
            realm_id: self.realm_id,
            checkpoint_id: realm_result.checkpoint_id,
            guta_stats: realm_result.guta_stats,
            top_line_proof: realm_result.top_line_proof,
            checkpoint_tree_root: realm_result.checkpoint_tree_root,
            circuit_type: realm_result.proof_id.circuit_type,
        };
        // Submit with retry
        self.submit_with_retry(input, proof).await
    }

    /// Get bytes from proof store with retry mechanism
    async fn get_bytes_with_retry<PS: QProofStoreAsyncImm>(
        &self,
        proof_store: Arc<PS>,
        job_id: QProvingJobDataID,
    ) -> Result<Vec<u8>> {
        self.retry_with_backoff("get_bytes_by_id", || async {
            match proof_store.get_bytes_by_id(job_id).await {
                Ok(bytes) if !bytes.is_empty() => Ok(bytes),
                Ok(_) => {
                    Err(anyhow!("empty bytes"))
                }
                Err(err) => Err(err),
            }
        }).await
    }

    /// Get proof from proof store with retry mechanism
    async fn get_proof_with_retry<PS: QProofStoreAsyncImm>(
        &self,
        proof_store: Arc<PS>,
        proof_id: QProvingJobDataID,
    ) -> Result<ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>> {
        self.retry_with_backoff("get_proof_by_id", || async {
            proof_store.get_proof_by_id(proof_id).await
        }).await
    }

    /// Submit request to coordinator with retry mechanism
    async fn submit_with_retry(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
        proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
    ) -> Result<()> {
        self.retry_with_backoff("submit_guta_proof", || async {
            info!("Sending job to coordinator");
            let params = rpc_params![input.clone(), proof.clone()];
            match self.http_client.request::<String, _>("qed_submit_guta", params).await {
                Ok(result) => {
                    info!("Successfully submitted job to coordinator, result: {}", result);
                    Ok(())
                }
                Err(err) => Err(err),
            }
        }).await
    }
}

impl Retryable for RealmProofSender {}
