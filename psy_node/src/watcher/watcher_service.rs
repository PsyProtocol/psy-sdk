use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::{DateTime, Utc};
use psy_core::job::id::QProvingJobDataID;
use psy_store::{node::{coordinator::PsyCoordinatorStoreReaderAsync, realm::PsyRealmStoreReaderAsync}, queue::{new_redis_async_pool, task_queue::QProvingTaskStoreImpl, QueueId, RsmqQueue}, store, store::PsyStore};
use redis::AsyncCommands;
use rsmq::RsmqMessage;
use tokio::{
    sync::Semaphore,
    time::{interval, timeout},
};
use tracing::{debug, error, info, warn};

use crate::watcher::{
    api_client::ApiClient,
    block_height::BlockHeightManager,
    common::*,
    config::WatcherConfig,
    events::WatcherMessage,
    schedule_tasks::{ExecutionTrigger, ScheduledTask, ScheduledTaskManager, TaskType},
    watcher::{NodeInfo, NodeType, TimeoutWatcher},
};

const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_ATTEMPT_TTL: u64 = 3600;
const TASK_MONITOR_INTERVAL: u64 = 5;
const BLOCK_SYNC_TIMEOUT: u64 = 10;
const FAILURE_BACKOFF_THRESHOLD: u32 = 3;
const FAILURE_BACKOFF_DURATION: u64 = 30;

pub struct WatcherService {
    config: WatcherConfig,
    psy_store: Arc<PsyStore>,
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    rsmq_queue: Arc<RsmqQueue>,
    rsmq_queue_id: QueueId,
    api_client: Arc<ApiClient>,
    block_height_manager: Arc<BlockHeightManager>,
    task_manager: Arc<ScheduledTaskManager>,
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

        let psy_store = Arc::new(
            store::from_backend(config.backend.to_backend())
                .await
                .map_err(|e| anyhow!("Database initialization failed: {}", e))?,
        );

        let redis_pool = Arc::new(
            new_redis_async_pool(&config.redis_uri, config.redis_pool_size)
                .await
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

        let rsmq_queue_id = QueueId::WatcherEvent { queue_biz_key: queue_name };
        rsmq_queue.create_queue_if_not_exists(&rsmq_queue_id).await?;

        let realm_id = config.node_type.eq(&NodeType::Realm).then(|| config.node_id.parse()).transpose()?;

        let api_client = Arc::new(ApiClient::new(
            config.api_endpoint.clone(),
            config.node_id.clone(),
            config.node_type,
            realm_id,
        )?);

        let block_height_manager = Arc::new(BlockHeightManager::new());

        match fetch_initial_block_height(&config.node_type, &psy_store).await {
            Ok(height) => {
                block_height_manager.set_height(height);
                info!("Block height initialized to {} from database", height);
            }
            Err(e) => warn!("Failed to fetch initial block height: {}. Continuing with height 0", e),
        }

        let task_manager = Arc::new(ScheduledTaskManager::new(redis_pool.clone(), config.node_id.clone()).await?);

        Ok(Self {
            node_info,
            redis_pool,
            rsmq_queue,
            rsmq_queue_id,
            api_client,
            block_height_manager,
            task_manager,
            timeout_watcher,
            config,
            psy_store,
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        info!("Starting Watcher Service for node: {}", self.config.node_id);

        tokio::select! {
            result = self.clone().process_messages() => {
                error!("Message processor stopped: {:?}", result);
                result
            }
            result = self.clone().monitor_scheduled_tasks() => {
                error!("Task monitor stopped: {:?}", result);
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
            .receive_message_with_id(&self.rsmq_queue_id, Some(MAX_SINGLE_MESSAGE_PROCESSING_TIME_SECS))
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

    async fn handle_processing_failure(&self, msg_id: &str, message: &WatcherMessage, attempts: u32, error: anyhow::Error) {
        let attempt_count = attempts + 1;
        error!("Failed to process message {} (attempt {}): {}", msg_id, attempt_count, error);

        if attempt_count >= MAX_RETRY_ATTEMPTS {
            self.send_to_dead_letter_queue(msg_id, message, error).await;
            return;
        }

        self.increment_message_attempts(msg_id).await;
    }

    async fn send_to_dead_letter_queue(&self, msg_id: &str, message: &WatcherMessage, error: anyhow::Error) {
        warn!("Message {} failed {} times, moving to dead letter queue", msg_id, MAX_RETRY_ATTEMPTS);

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
                info!("UserEvent: user registration with pk ({})", event.public_key);
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

    async fn handle_backup_proof(&self, event: &crate::watcher::events::BackupProofEvent) -> Result<()> {
        info!("Processing proof backup: {:?}", event.job_id);

        self.report_with_retry(|| self.api_client.send_proof_backup(event.clone()), 3, Duration::from_secs(1))
            .await?;

        self.schedule_deletion(event.job_id.clone(), TaskType::DeleteProof, event.delete_after_blocks)
            .await
    }

    async fn handle_backup_witness(&self, event: &crate::watcher::events::BackupWitnessEvent) -> Result<()> {
        info!("Processing witness backup: {:?}", event.job_id);

        self.report_with_retry(|| self.api_client.send_witness_backup(event.clone()), 3, Duration::from_secs(1))
            .await?;

        self.schedule_deletion(event.job_id.clone(), TaskType::DeleteWitness, event.delete_after_blocks)
            .await
    }

    async fn monitor_scheduled_tasks(self: Arc<Self>) -> Result<()> {
        info!("Starting scheduled task monitor");
        let mut ticker = interval(Duration::from_secs(TASK_MONITOR_INTERVAL));

        loop {
            ticker.tick().await;
            self.process_ready_tasks().await?;
        }
    }

    async fn process_ready_tasks(&self) -> Result<()> {
        let current_height = self.block_height_manager.get_height();
        let current_time = current_timestamp();
        let ready_tasks = self.task_manager.get_ready_tasks(current_height, current_time).await?;

        for task in ready_tasks {
            info!("Processing scheduled task: {}", task.task_id);

            if let Err(e) = self.execute_scheduled_task(&task).await {
                error!("Failed to execute task {}: {}", task.task_id, e);
                self.task_manager.retry_task(task).await?;
                continue;
            }

            info!("Successfully executed task: {}", task.task_id);
            self.task_manager.complete_task(&task.task_id).await?;
        }

        Ok(())
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

    async fn schedule_deletion(&self, job_id: QProvingJobDataID, task_type: TaskType, blocks_to_wait: u64) -> Result<()> {
        let current_height = self.block_height_manager.get_height();
        let target_height = current_height + blocks_to_wait;

        let task_prefix = match task_type {
            TaskType::DeleteProof => "delete_proof",
            TaskType::DeleteWitness => "delete_witness",
            _ => "delete",
        };

        let task = ScheduledTask {
            task_id: format!("{}_{}", task_prefix, job_id),
            task_type,
            execute_at: ExecutionTrigger::AfterBlocks { target_height },
            payload: serde_json::json!({ "job_id": job_id }),
            created_at: current_timestamp(),
            retry_count: 0,
        };

        self.task_manager.schedule_task(task).await?;
        info!("Scheduled deletion at block height {}", target_height);
        Ok(())
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
            NodeType::Coordinator => PsyCoordinatorStoreReaderAsync::get_latest_block_state(&self.psy_store).await?,
            NodeType::Realm => PsyRealmStoreReaderAsync::get_latest_block_state(&self.psy_store).await?,
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
}

async fn fetch_initial_block_height(node_type: &NodeType, store: &PsyStore) -> Result<u64> {
    let block_state = match node_type {
        NodeType::Coordinator => PsyCoordinatorStoreReaderAsync::get_latest_block_state(store).await?,
        NodeType::Realm => PsyRealmStoreReaderAsync::get_latest_block_state(store).await?,
    };

    Ok(block_state.checkpoint_id)
}

pub fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}
pub fn current_timestamp_mills() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

pub fn current_datetime() -> DateTime<Utc> {
    Utc::now()
}
