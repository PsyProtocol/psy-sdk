use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::{DateTime, Utc};
use qed_core::{config::network_constants::SLOT_SIZE, job::id::QProvingJobDataID};
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::task_queue::QProvingTaskStoreImpl;
use qed_store::queue::{new_redis_async_pool, QueueId, RsmqQueue};
use qed_store::store::QEDStore;
use redis::AsyncCommands;
use rsmq::RsmqMessage;
use serde::{Deserialize, Serialize};
use tokio::sync::{Semaphore, RwLock};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};
use qed_data::config::store_config::QEDFelt;
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use crate::watcher::{api_client::ApiClient, block_height::BlockHeightManager, common::*, config::WatcherConfig, events::WatcherMessage, schedule_tasks::{ExecutionTrigger, ScheduledTask, ScheduledTaskManager, TaskType}, watcher::{NodeInfo, WatcherSourceNodeType, TimeoutWatcher}, ApiClientConfig};

const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_ATTEMPT_TTL: u64 = 3600;
const TASK_MONITOR_INTERVAL: u64 = 5;
const BLOCK_SYNC_TIMEOUT: u64 = 10;
const FAILURE_BACKOFF_THRESHOLD: u32 = 3;
const FAILURE_BACKOFF_DURATION: u64 = 30;
const BLOCK_METADATA_WAIT_BLOCK_NUM: u64 = 3;
const BLOCK_METADATA_WAIT_DURATION: u64 = BLOCK_METADATA_WAIT_BLOCK_NUM * SLOT_SIZE;

const CHECKPOINT_LEAF_FETCH_RETRY_COUNT: u32 = 3;
const CHECKPOINT_LEAF_FETCH_RETRY_DELAY: u64 = 5;

pub struct WatcherService {
    config: WatcherConfig,
    qed_store: Arc<QEDStore>,
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    rsmq_queue: Arc<RsmqQueue>,
    rsmq_queue_id: QueueId,
    api_client: Arc<ApiClient>,
    block_height_manager: Arc<BlockHeightManager>,
    block_metadata_height: Arc<AtomicU64>,
    timeout_watcher: Arc<TimeoutWatcher>,
    node_info: Arc<NodeInfo>,
}

impl WatcherService {
    pub async fn new(config: WatcherConfig) -> Result<Self> {
        info!("Initializing watcher service");

        let node_info = Arc::new(NodeInfo {
            node_id: config.node_id.clone(),
            node_type: config.node_type,
        });

        let qed_store = Arc::new(
        QEDStore::from_backend(config.backend.to_backend()).await
            .map_err(|e| anyhow!("Database initialization failed: {}", e))?,
        );

        let redis_pool = Arc::new(
        new_redis_async_pool(&config.redis_uri, config.redis_pool_size).await
            .map_err(|e| anyhow!("Failed to create Redis pool: {}", e))?,
        );

        let queue_name = get_queue_name(&config.queue_id.queue_biz_key);

        let rsmq_queue = Arc::new(
            RsmqQueue::new(&config.redis_uri, config.redis_pool_size, &queue_name)
                .await
                .map_err(|e| anyhow!("Failed to create RSMQ queue: {}", e))?,
        );

        let timeout_watcher = Arc::new(TimeoutWatcher::new(
            redis_pool.clone(),
            config.redis_uri.clone(),
            rsmq_queue.clone(),
            node_info.clone(),
            &queue_name,
        ));

        let rsmq_queue_id = QueueId::WatcherEvent {
            queue_biz_key: queue_name,
        };
        rsmq_queue.create_queue_if_not_exists(&rsmq_queue_id).await?;

        let realm_id = config.node_type
            .eq(&WatcherSourceNodeType::Realm)
            .then(|| config.node_id.parse())
            .transpose()?;

        // Initialize API client with JWT support
        let mut api_client_config = ApiClientConfig::new(
            config.api_endpoint.clone(),
            config.node_id.clone(),
            config.node_type,
            realm_id,
        );

        // Add JWT secret if configured
        if let Some(jwt_secret) = config.jwt_secret.clone() {
            info!("Configuring API client with JWT authentication for telemetry endpoints");
            api_client_config = api_client_config.with_jwt_secret(jwt_secret);
        } else {
            warn!(
                "API client initialized without JWT authentication. \
                Telemetry endpoints may fail. Set JWT_SECRET environment variable."
            );
        }

        let api_client = Arc::new(ApiClient::with_config(api_client_config)?);

        let block_height_manager = Arc::new(BlockHeightManager::new());

        let height =  match fetch_initial_block_height(&config.node_type, &qed_store).await {
            Ok(height) => {
                block_height_manager.set_height(height);
                info!("Block height initialized to {} from database", height);
                height
            }
            Err(e) => {
                warn!("Failed to fetch initial block height: {}. Continuing with height 0", e);
                0
            }
        };
        let block_metadata_height = Arc::new(AtomicU64::new(0));


        Ok(Self {
            node_info,
            redis_pool,
            rsmq_queue,
            rsmq_queue_id,
            api_client,
            block_height_manager,
            block_metadata_height,
            timeout_watcher,
            config,
            qed_store,
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        info!("Starting Watcher Service for node: {}", self.config.node_id);

        tokio::select! {
            result = self.clone().process_messages() => {
                error!("Message processor stopped: {:?}", result);
                result
            }
            result = self.clone().sync_block_height() => {
                error!("Block sync stopped: {:?}", result);
                result
            }
            result = self.clone().monitor_timeouts() => {
                error!("Timeout monitor stopped: {:?}", result);
                result
            }
            result = self.clone().send_checkpoint_leaves() => {
                error!("Send block metadata stopped: {:?}", result);
                result
            }
        }
    }

    async fn process_messages(self: Arc<Self>) -> Result<()> {
        info!("Starting message processor");

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));
        let active_tasks = Arc::new(AtomicUsize::new(0));

        loop {
            let current_active = active_tasks.load(Ordering::Relaxed);

            if self.is_at_capacity(current_active).await {
                continue;
            }

            let msg = match self.receive_next_message().await {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    self.wait_for_messages(current_active).await;
                    continue;
                }
                Err(e) => {
                    error!("Failed to receive message: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            self.spawn_message_handler(msg, &semaphore, &active_tasks).await?;
        }
    }

    async fn is_at_capacity(&self, current_active: usize) -> bool {
        if current_active < MAX_CONCURRENT_TASKS {
            return false;
        }

        debug!("At max capacity ({} tasks), waiting...", current_active);
        tokio::time::sleep(Duration::from_millis(10)).await;
        true
    }

    async fn receive_next_message(&self) -> Result<Option<RsmqMessage<Vec<u8>>>> {
        self.rsmq_queue
            .receive_message_with_id(
                &self.rsmq_queue_id,
                Some(MAX_SINGLE_MESSAGE_PROCESSING_TIME_SECS)
            )
            .await
    }

    async fn wait_for_messages(&self, current_active: usize) {
        let delay_ms = if current_active == 0 {
            SLEEP_TIME_IF_NO_MSG_MILLIS
        } else {
            SLEEP_TIME_IF_HAVE_MSG_MILLIS
        };
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    async fn spawn_message_handler(
        self: &Arc<Self>,
        msg: RsmqMessage<Vec<u8>>,
        semaphore: &Arc<Semaphore>,
        active_tasks: &Arc<AtomicUsize>,
    ) -> Result<()> {
        let permit = semaphore.clone().acquire_owned().await?;
        let self_clone = self.clone();
        let active_tasks_clone = active_tasks.clone();

        active_tasks.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            let _permit = permit;
            self_clone.process_single_message(msg).await;
            active_tasks_clone.fetch_sub(1, Ordering::Relaxed);
        });

        Ok(())
    }

    async fn process_single_message(self: Arc<Self>, msg: RsmqMessage<Vec<u8>>) {
        let msg_id = &msg.id;

        let message = match bincode::deserialize::<WatcherMessage>(&msg.message) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to deserialize message {}: {}", msg_id, e);
                self.delete_message(msg_id).await;
                return;
            }
        };

        debug!("Processing message {} in spawned task", msg_id);
        let attempts = self.get_message_attempts(msg_id).await.unwrap_or(0);

        if let Err(e) = self.handle_message(&message).await {
            self.handle_processing_failure(msg_id, &message, attempts, e).await;
            return;
        }

        self.complete_message(msg_id).await;
    }

    async fn complete_message(&self, msg_id: &str) {
        if let Err(e) = self.rsmq_queue.delete_message(&self.rsmq_queue_id, msg_id).await {
            error!("Failed to delete message {}: {}", msg_id, e);
            return;
        }

        debug!("Successfully processed and deleted message {}", msg_id);
        self.clear_redis_key(&format!("watcher:msg_attempts:{}", msg_id)).await;
    }

    async fn handle_processing_failure(
        &self,
        msg_id: &str,
        message: &WatcherMessage,
        attempts: u32,
        error: anyhow::Error,
    ) {
        let attempt_count = attempts + 1;
        error!("Failed to process message {} (attempt {}): {}", msg_id, attempt_count, error);

        if attempt_count >= MAX_RETRY_ATTEMPTS {
            self.send_to_dead_letter_queue(msg_id, message, error).await;
            return;
        }

        self.increment_message_attempts(msg_id).await;
    }

    async fn send_to_dead_letter_queue(&self, msg_id: &str, message: &WatcherMessage, error: anyhow::Error) {
        warn!(
            "Message {} failed {} times, moving to dead letter queue",
            msg_id,
            MAX_RETRY_ATTEMPTS
        );

        self.move_to_dead_letter_queue(msg_id, message.clone(), error.to_string()).await;
        self.clear_redis_key(&format!("watcher:msg_attempts:{}", msg_id)).await;
    }

    async fn delete_message(&self, msg_id: &str) {
        let _ = self.rsmq_queue.delete_message(&self.rsmq_queue_id, msg_id).await;
    }

    async fn handle_message(&self, message: &WatcherMessage) -> Result<()> {
        use WatcherMessage::*;

        match message {
            UserRegistration(event) => {
                info!("UserEvent: user registration with pk ({})", event.metadata.public_key);
                self.api_client.send_user_registration(event.clone()).await
            }
            DeployContract(event) => {
                info!("UserEvent: contract deployment, deployer: {}", event.deployer);
                self.api_client.send_contract_deployment(event.clone()).await
            }
            GutaSubmission(event) => {
                info!(
                    "UserEvent: GUTA submission from realm: {}, circuit type {}",
                    event.realm_id, event.metadata.circuit_type
                );
                self.api_client.send_guta_submission(event.clone()).await
            }
            EndcapSubmission(event) => {
                info!(
                    "UserEvent: Endcap submission from realm: {}, user {}, start_user_leaf_hash {}, end_user_leaf_hash{}",
                    event.realm_id,
                    event.user_id,
                    event.metadata.state_transition.start_user_leaf_hash,
                    event.metadata.state_transition.end_user_leaf_hash,
                );
                self.api_client.send_endcap_submission(event.clone()).await
            }

            JobPending(event) => {
                info!("JobEvent: pending: {:?}", event.job_id);
                self.api_client.send_job_pending(event.clone()).await
            }
            JobStarted(event) => {
                info!("JobEvent: started: {:?} by worker {}", event.job_id, event.worker_id);
                self.api_client.send_job_started(event.clone()).await
            }
            JobCompleted(event) => {
                info!("JobEvent: completed: {:?} by worker {:?}", event.job_id, event.worker_id);
                self.api_client.send_job_completed(event.clone()).await
            }
            JobTimeout(event) => {
                warn!("JobEvent: timeout {:?}", event.job_id);
                self.api_client.send_job_timeout(event.clone()).await
            }
        }
    }


    async fn sync_block_height(self: Arc<Self>) -> Result<()> {
        info!("Starting block height synchronization");

        let mut ticker = interval(Duration::from_secs(self.config.block_sync_interval));
        let mut consecutive_failures = 0;

        loop {
            ticker.tick().await;
            consecutive_failures = self.sync_single_block_height(consecutive_failures).await;

            if consecutive_failures > 0 && consecutive_failures % FAILURE_BACKOFF_THRESHOLD == 0 {
                debug!("Backing off for {}s due to failures", FAILURE_BACKOFF_DURATION);
                tokio::time::sleep(Duration::from_secs(FAILURE_BACKOFF_DURATION)).await;
            }
        }
    }

    async fn sync_single_block_height(&self, failures: u32) -> u32 {
        match timeout(Duration::from_secs(BLOCK_SYNC_TIMEOUT), self.fetch_block_height_from_db()).await {
            Ok(Ok(new_height)) => {
                self.block_height_manager.update_height(new_height);
                0
            }
            Ok(Err(e)) => {
                error!("Failed to fetch block height (attempt {}): {}", failures + 1, e);
                failures + 1
            }
            Err(_) => {
                error!("Timeout fetching block height (attempt {})", failures + 1);
                failures + 1
            }
        }
    }

    async fn monitor_timeouts(self: Arc<Self>) -> Result<()> {
        info!("Starting timeout monitor");
        self.timeout_watcher.start_monitoring().await
    }

    async fn report_with_retry<F, Fut>(&self, f: F, max_retries: u32, base_delay: Duration) -> Result<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        for attempts in 1..=max_retries {
            if let Err(e) = f().await {
                if attempts >= max_retries {
                    error!("Failed after {} attempts: {}", max_retries, e);
                    return Err(e);
                }

                let delay = base_delay * attempts;
                warn!("Attempt {} failed: {}, retrying in {:?}", attempts, e, delay);
                tokio::time::sleep(delay).await;
                continue;
            }

            if attempts > 1 {
                info!("Successfully reported after {} attempts", attempts);
            }
            return Ok(());
        }

        unreachable!()
    }

    async fn execute_scheduled_task(&self, task: &ScheduledTask) -> Result<()> {
        info!("Executing scheduled task: {} ({:?})", task.task_id, task.task_type);

        let job_id = self.extract_job_id_from_task(task).await?;

        match &task.task_type {
            TaskType::DeleteProof => self.delete_redis_key(&format!("proof:{}", job_id)).await,
            TaskType::DeleteWitness => self.delete_redis_key(&format!("witness:{}", job_id)).await,
            TaskType::Custom(name) => self.execute_custom_task(name).await,
        }
    }

    async fn extract_job_id_from_task(&self, task: &ScheduledTask) -> Result<QProvingJobDataID> {
        task.payload
            .get("job_id")
            .ok_or_else(|| anyhow!("Missing job_id in task payload"))
            .and_then(|v| serde_json::from_value(v.clone()).map_err(Into::into))
    }

    async fn execute_custom_task(&self, name: &str) -> Result<()> {
        match name {
            "health_check" => self.perform_health_check().await,
            "sync_data" => self.sync_data_with_datacenter().await,
            _ => {
                warn!("Unknown custom task: {}", name);
                Ok(())
            }
        }
    }

    async fn fetch_block_height_from_db(&self) -> Result<u64> {
        let block_state = match self.config.node_type {
            WatcherSourceNodeType::Coordinator => {
                QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&self.qed_store).await?
            }
            WatcherSourceNodeType::Realm => {
                QEDRealmStoreReaderAsync::get_latest_l2_block_state(&self.qed_store).await?
            }
        };

        Ok(block_state.checkpoint_id)
    }

    async fn get_message_attempts(&self, msg_id: &str) -> Result<u32> {
        let key = format!("watcher:msg_attempts:{}", msg_id);
        Ok(self.redis_pool.get().await?.get(&key).await.unwrap_or(0))
    }

    async fn increment_message_attempts(&self, msg_id: &str) {
        let key = format!("watcher:msg_attempts:{}", msg_id);

        let Ok(mut conn) = self.redis_pool.get().await else {
            return;
        };

        let attempts = conn.get::<_, u32>(&key).await.unwrap_or(0) + 1;
        let _: Result<(), redis::RedisError> = conn.set_ex(&key, attempts, RETRY_ATTEMPT_TTL).await;
    }

    async fn clear_redis_key(&self, key: &str) {
        let Ok(mut conn) = self.redis_pool.get().await else {
            return;
        };

        let _: Result<i32, redis::RedisError> = conn.del(key).await;
    }


    async fn delete_redis_key(&self, key: &str) -> Result<()> {
        let deleted: i32 = self.redis_pool.get().await?.del(key).await?;

        if deleted > 0 {
            info!("Deleted Redis key: {}", key);
        } else {
            warn!("Redis key not found: {}", key);
        }
        Ok(())
    }

    async fn move_to_dead_letter_queue(&self, msg_id: &str, message: WatcherMessage, error: String) {
        let dlq_id = QueueId::WorkerEvent {
            queue_biz_key: format!("{}_dlq", self.rsmq_queue_id.get_queue_id()),
        };

        let _ = self.rsmq_queue.create_queue_if_not_exists(&dlq_id).await;

        let dlq_message = serde_json::json!({
            "original_msg_id": msg_id,
            "message": message,
            "error": error,
            "failed_at": current_timestamp(),
            "node_id": self.config.node_id,
        });

        let serialized = match serde_json::to_vec(&dlq_message) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to serialize DLQ message: {}", e);
                return;
            }
        };

        match self.rsmq_queue.send_message(&dlq_id, serialized).await {
            Ok(_) => {
                info!("Moved message {} to dead letter queue", msg_id);
                self.delete_message(msg_id).await;
            }
            Err(e) => error!("Failed to move message {} to DLQ: {}", msg_id, e),
        }
    }

    async fn perform_health_check(&self) -> Result<()> {
        let redis_health = self.timeout_watcher.check_redis_health().await?;
        let block_height = self.block_height_manager.get_height();
        info!("Health check: Redis={}, BlockHeight={}", redis_health, block_height);
        Ok(())
    }

    async fn sync_data_with_datacenter(&self) -> Result<()> {
        info!("Syncing data with datacenter");
        Ok(())
    }

    async fn send_checkpoint_leaves(self: Arc<Self>) -> Result<()> {
        info!("Starting sending checkpoint leaves to api service");

        loop {
            tokio::time::sleep(Duration::from_millis(BLOCK_METADATA_WAIT_DURATION)).await;

            //the finalized height is latest_height - BLOCK_METADATA_WAIT_BLOCK_NUM
            let latest_height = self.fetch_block_height_from_db().await?;
            let finalized_height = latest_height.saturating_sub(BLOCK_METADATA_WAIT_BLOCK_NUM);

            //watcher local height
            let local_height = self.block_metadata_height.load(Ordering::Relaxed);

            if finalized_height <= local_height {
                debug!(
                    "finalized height({}) <= local height ({}), sleep {} s",
                    finalized_height,
                    local_height,
                    BLOCK_METADATA_WAIT_DURATION / 1000
                );
                continue;
            }
            let mut checkpoint_leaves = Vec::new();
            let mut fetch_failed = false;

            for checkpoint_id in local_height..finalized_height {
                let mut last_error = None;
                let mut checkpoint_leaf = None;

                // Retry mechanism for fetching checkpoint leaf data
                for attempt in 0..CHECKPOINT_LEAF_FETCH_RETRY_COUNT {
                    if attempt > 0 {
                        warn!(
                            "Retrying to fetch checkpoint {} leaf (attempt {}/{})",
                            checkpoint_id,
                            attempt + 1,
                            CHECKPOINT_LEAF_FETCH_RETRY_COUNT
                        );
                        tokio::time::sleep(Duration::from_secs(CHECKPOINT_LEAF_FETCH_RETRY_DELAY)).await;
                    }

                    match QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data(&self.qed_store, checkpoint_id).await {
                        Ok(leaf) => {
                            checkpoint_leaf = Some(leaf);
                            break;
                        }
                        Err(e) => {
                            last_error = Some(e);
                            warn!(
                                "Failed to fetch checkpoint {} leaf (attempt {}/{}): {}",
                                checkpoint_id,
                                attempt + 1,
                                CHECKPOINT_LEAF_FETCH_RETRY_COUNT,
                                last_error.as_ref().unwrap()
                            );
                        }
                    }
                }

                // If all retries failed, skip this execution and return to main loop
                if checkpoint_leaf.is_none() {
                    error!(
                        "Failed to fetch checkpoint {} leaf after {} attempts: {}. Skipping this execution.",
                        checkpoint_id,
                        CHECKPOINT_LEAF_FETCH_RETRY_COUNT,
                        last_error.unwrap()
                    );
                    fetch_failed = true;
                    break;
                }

                let leaf = checkpoint_leaf.unwrap();
                debug!(
                    "checkpoint {} leaf: {}",
                    checkpoint_id,
                    serde_json::to_string_pretty(&leaf.stats)?
                );

                let cl = CheckpointLeafWithId {
                    checkpoint_id,
                    checkpoint_leaf: leaf,
                };
                checkpoint_leaves.push(cl);
            }

            // If fetch failed, skip sending and continue to next iteration of main loop
            if fetch_failed {
                warn!(
                    "Skipping block metadata send due to checkpoint fetch failure. Will retry in next iteration."
                );
                continue;
            }

            // Send the collected checkpoint leaves
            match self.api_client.send_checkpoint_leaves(checkpoint_leaves).await {
                Ok(_) => {
                    // Only update local height after successful send
                    self.block_metadata_height.store(finalized_height, Ordering::Relaxed);
                    info!(
                        "Successfully sent checkpoint leaves from {} to {}",
                        local_height,
                        finalized_height
                    );
                }
                Err(e) => {
                    error!("Failed to send block metadata to API service: {}", e);
                }
            }

        }
    }
}

async fn fetch_initial_block_height(node_type: &WatcherSourceNodeType, store: &QEDStore) -> Result<u64> {

    let block_state = match node_type {
        WatcherSourceNodeType::Coordinator => {
            QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(store).await?
        }
        WatcherSourceNodeType::Realm => {
            QEDRealmStoreReaderAsync::get_latest_l2_block_state(store).await?
        }
    };

    Ok(block_state.checkpoint_id)
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
pub fn current_timestamp_mills() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn current_datetime() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointLeafWithId {
    pub checkpoint_id: u64,
    pub checkpoint_leaf: QEDCheckpointLeaf<QEDFelt>,
}