use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::{DateTime, Utc};
use qed_core::job::id::QProvingJobDataID;
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::task_queue::QProvingTaskStoreImpl;
use qed_store::queue::{new_redis_async_pool, QueueId, RsmqQueue};
use qed_store::store::QEDStore;
use redis::AsyncCommands;
use rsmq::RsmqMessage;
use tokio::sync::Semaphore;
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

use crate::watcher::{
    api_client::ApiClient,
    block_height::BlockHeightManager,
    config::WatcherConfig,
    events::WatcherMessage,
    schedule_tasks::{ExecutionTrigger, ScheduledTask, ScheduledTaskManager, TaskType},
    watcher::{NodeInfo, NodeType, TimeoutWatcher},
};

pub const WATCHER_RSMQ: &str = "watcher_rsmq";

pub struct WatcherService {
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    rsmq_queue: Arc<RsmqQueue>,
    rsmq_queue_id: QueueId,
    api_client: Arc<ApiClient>,
    block_height_manager: Arc<BlockHeightManager>,
    task_manager: Arc<ScheduledTaskManager>,
    timeout_watcher: Arc<TimeoutWatcher>,
    config: WatcherConfig,
    node_info: Arc<NodeInfo>,
    qed_store: Arc<QEDStore>,
}

impl WatcherService {
    const MAX_CONCURRENT_TASKS: usize = 100;
    const MAX_RETRY_ATTEMPTS: u32 = 3;
    const JOB_STATUS_CLEANUP_BLOCKS: u64 = 128;

    pub async fn new(config: WatcherConfig) -> Result<Self> {
        info!("Initializing watcher service");

        let node_info = Arc::new(NodeInfo {
            node_id: config.node_id.clone(),
            node_type: config.node_type,
        });

        let qed_store = Arc::new(
            QEDStore::from_backend(config.backend.to_backend())
                .await
                .map_err(|e| anyhow!("Database initialization failed: {}", e))?,
        );

        let redis_pool = Arc::new(
            new_redis_async_pool(&config.redis_url, config.redis_pool_size)
                .await
                .map_err(|e| anyhow!("Failed to create Redis pool: {}", e))?,
        );

        let rsmq_queue = Arc::new(
            RsmqQueue::new(&config.redis_url, config.redis_pool_size, WATCHER_RSMQ)
                .await
                .map_err(|e| anyhow!("Failed to create RSMQ queue: {}", e))?,
        );

        let rsmq_queue_id = QueueId::WatcherEvent {
            queue_biz_key: WATCHER_RSMQ.to_string(),
        };
        rsmq_queue.create_queue_if_not_exists(&rsmq_queue_id).await?;

        let realm_id = (config.node_type == NodeType::Realm)
            .then(|| config.node_id.parse())
            .transpose()?;

        let api_client = Arc::new(ApiClient::new(
            config.api_endpoint.clone(),
            config.node_id.clone(),
            config.node_type,
            realm_id,
        )?);

        let block_height_manager = Arc::new(BlockHeightManager::new());

        // Initialize block height from database
        match fetch_initial_block_height(&config.node_type, &qed_store).await {
            Ok(height) => {
                block_height_manager.set_height(height);
                info!("Block height initialized to {} from database", height);
            }
            Err(e) => warn!("Failed to fetch initial block height: {}. Continuing with height 0", e),
        }

        let task_manager = Arc::new(
            ScheduledTaskManager::new(redis_pool.clone(), config.node_id.clone()).await?,
        );

        let timeout_watcher = Arc::new(TimeoutWatcher::new(
            redis_pool.clone(),
            config.redis_url.clone(),
            rsmq_queue.clone(),
            node_info.clone(),
            WATCHER_RSMQ.to_string(),
        ));

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

        let semaphore = Arc::new(Semaphore::new(Self::MAX_CONCURRENT_TASKS));
        let active_tasks = Arc::new(AtomicU32::new(0));

        loop {
            let current_active = active_tasks.load(Ordering::Acquire);
            if current_active >= Self::MAX_CONCURRENT_TASKS as u32 {
                debug!("At max capacity ({} tasks), waiting...", current_active);
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }

            match self
                .rsmq_queue
                .receive_message_with_id(&self.rsmq_queue_id, Some(Duration::from_secs(30)))
                .await
            {
                Ok(Some(msg)) => {
                    let permit = semaphore.clone().acquire_owned().await?;
                    let self_clone = self.clone();
                    let active_tasks_clone = active_tasks.clone();

                    active_tasks.fetch_add(1, Ordering::Release);

                    tokio::spawn(async move {
                        let _permit = permit;
                        self_clone.process_single_message(msg).await;
                        active_tasks_clone.fetch_sub(1, Ordering::Release);
                    });
                }
                Ok(None) => {
                    let delay = if current_active == 0 { 100 } else { 10 };
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => {
                    error!("Failed to receive message: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn process_single_message(self: Arc<Self>, msg: RsmqMessage<Vec<u8>>) {
        let msg_id = &msg.id;

        let message = match bincode::deserialize::<WatcherMessage>(&msg.message) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to deserialize message {}: {}", msg_id, e);
                let _ = self.rsmq_queue.delete_message(&self.rsmq_queue_id, msg_id).await;
                return;
            }
        };

        debug!("Processing message {} in spawned task", msg_id);
        let attempts = self.get_message_attempts(msg_id).await.unwrap_or(0);

        match self.handle_message(message.clone()).await {
            Ok(_) => {
                if let Err(e) = self.rsmq_queue.delete_message(&self.rsmq_queue_id, msg_id).await {
                    error!("Failed to delete message {}: {}", msg_id, e);
                } else {
                    debug!("Successfully processed and deleted message {}", msg_id);
                }
                let _ = self.clear_message_attempts(msg_id).await;
            }
            Err(e) => {
                error!("Failed to process message {} (attempt {}): {}", msg_id, attempts + 1, e);

                if attempts + 1 >= Self::MAX_RETRY_ATTEMPTS {
                    warn!("Message {} failed {} times, moving to dead letter queue", msg_id, Self::MAX_RETRY_ATTEMPTS);
                    self.move_to_dead_letter_queue(msg_id, message, e.to_string()).await;
                    let _ = self.clear_message_attempts(msg_id).await;
                } else {
                    let _ = self.record_message_attempt(msg_id).await;
                }
            }
        }
    }

    async fn handle_message(&self, message: WatcherMessage) -> Result<()> {
        use WatcherMessage::*;

        match message {
            UserRegistration(event) => {
                info!("UserEvent: user registration with pk ({})", event.public_key);
                self.api_client.send_user_registration(event).await?;
            }
            DeployContract(event) => {
                info!("UserEvent: contract deployment, deployer: {}", event.deployer);
                self.api_client.send_contract_deployment(event).await?;
            }
            GutaSubmission(event) => {
                info!("UserEvent: GUTA submission from realm: {}, circuit type {}",
                    event.realm_id, event.metadata.circuit_type);
                self.api_client.send_guta_submission(event).await?;
            }
            JobStarted(event) => {
                info!("JobEvent: started: {:?} by worker {}", event.job_id, event.worker_id);
                self.api_client.send_job_started(event).await?;
            }
            JobCompleted(event) => {
                info!("JobEvent: completed: {:?} by worker {:?}", event.job_id, event.worker_id);

                // Schedule job status cleanup
                let current_height = self.block_height_manager.get_height();
                self.task_manager
                    .schedule_job_status_cleanup(
                        event.job_id.clone(),
                        Self::JOB_STATUS_CLEANUP_BLOCKS,
                        current_height,
                    )
                    .await?;

                self.api_client.send_job_completed(event).await?;
            }
            JobTimeout(event) => {
                warn!("JobEvent: timeout {:?}", event.job_id);
                self.api_client.send_job_timeout(event).await?;
            }
            BackupProof(event) => {
                info!("Processing proof backup: {:?}", event.job_id);

                self.report_with_retry(
                    || self.api_client.send_proof_backup(event.clone()),
                    3,
                    Duration::from_secs(1),
                ).await?;

                self.schedule_deletion(
                    event.job_id,
                    TaskType::DeleteProof,
                    event.delete_after_blocks,
                ).await?;
            }
            BackupWitness(event) => {
                info!("Processing witness backup: {:?}", event.job_id);

                self.report_with_retry(
                    || self.api_client.send_witness_backup(event.clone()),
                    3,
                    Duration::from_secs(1),
                ).await?;

                self.schedule_deletion(
                    event.job_id,
                    TaskType::DeleteWitness,
                    event.delete_after_blocks,
                ).await?;
            }
            _ => debug!("Unhandled message type"),
        }
        Ok(())
    }

    async fn monitor_scheduled_tasks(self: Arc<Self>) -> Result<()> {
        info!("Starting scheduled task monitor");
        let mut ticker = interval(Duration::from_secs(5));

        loop {
            ticker.tick().await;

            let current_height = self.block_height_manager.get_height();
            let current_time = current_timestamp();
            let ready_tasks = self.task_manager.get_ready_tasks(current_height, current_time).await?;

            for task in ready_tasks {
                info!("Processing scheduled task: {}", task.task_id);

                match self.execute_scheduled_task(&task).await {
                    Ok(_) => {
                        info!("Successfully executed task: {}", task.task_id);
                        self.task_manager.complete_task(&task.task_id).await?;
                    }
                    Err(e) => {
                        error!("Failed to execute task {}: {}", task.task_id, e);
                        self.task_manager.retry_task(task).await?;
                    }
                }
            }
        }
    }

    async fn sync_block_height(self: Arc<Self>) -> Result<()> {
        info!("Starting block height synchronization");

        let mut ticker = interval(Duration::from_secs(self.config.block_sync_interval));
        let mut consecutive_failures = 0;

        loop {
            ticker.tick().await;

            match timeout(Duration::from_secs(10), self.fetch_block_height_from_db()).await {
                Ok(Ok(new_height)) => {
                    consecutive_failures = 0;
                    self.block_height_manager.update_height(new_height);
                }
                Ok(Err(e)) => {
                    consecutive_failures += 1;
                    error!("Failed to fetch block height (attempt {}): {}", consecutive_failures, e);
                }
                Err(_) => {
                    consecutive_failures += 1;
                    error!("Timeout fetching block height (attempt {})", consecutive_failures);
                }
            }

            if consecutive_failures > 0 && consecutive_failures % 3 == 0 {
                debug!("Backing off for 30s due to failures");
                tokio::time::sleep(Duration::from_secs(30)).await;
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
            match f().await {
                Ok(_) => {
                    if attempts > 1 {
                        info!("Successfully reported after {} attempts", attempts);
                    }
                    return Ok(());
                }
                Err(e) if attempts < max_retries => {
                    let delay = base_delay * attempts;
                    warn!("Attempt {} failed: {}, retrying in {:?}", attempts, e, delay);
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    error!("Failed after {} attempts: {}", max_retries, e);
                    return Err(e);
                }
            }
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

        let job_id_value = task.payload.get("job_id")
            .ok_or_else(|| anyhow!("Missing job_id in task payload"))?;
        let job_id: QProvingJobDataID = serde_json::from_value(job_id_value.clone())?;

        match &task.task_type {
            TaskType::DeleteProof => self.delete_proof(&job_id).await,
            TaskType::DeleteWitness => self.delete_witness(&job_id).await,
            TaskType::CleanupJob => self.cleanup_job_data(&job_id).await,
            TaskType::CleanupJobStatus => self.cleanup_job_status(&job_id).await,
            TaskType::Custom(name) => match name.as_str() {
                "health_check" => self.perform_health_check().await,
                "sync_data" => self.sync_data_with_datacenter().await,
                _ => {
                    warn!("Unknown custom task: {}", name);
                    Ok(())
                }
            },
        }
    }

    async fn fetch_block_height_from_db(&self) -> Result<u64> {
        let block_state = match self.config.node_type {
            NodeType::Coordinator => {
                QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&self.qed_store).await?
            }
            NodeType::Realm => {
                QEDRealmStoreReaderAsync::get_latest_l2_block_state(&self.qed_store).await?
            }
        };

        Ok(block_state.checkpoint_id)
    }

    async fn get_message_attempts(&self, msg_id: &str) -> Result<u32> {
        let key = format!("watcher:msg_attempts:{}", msg_id);
        Ok(self.redis_pool.get().await?.get(&key).await.unwrap_or(0))
    }

    async fn record_message_attempt(&self, msg_id: &str) -> Result<()> {
        let key = format!("watcher:msg_attempts:{}", msg_id);
        let attempts = self.get_message_attempts(msg_id).await? + 1;
        self.redis_pool.get().await?.set_ex(&key, attempts, 3600).await?;
        Ok(())
    }

    async fn clear_message_attempts(&self, msg_id: &str) -> Result<()> {
        let key = format!("watcher:msg_attempts:{}", msg_id);
        self.redis_pool.get().await?.del(&key).await?;
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

        match self.rsmq_queue.send_message(&dlq_id, serde_json::to_vec(&dlq_message).unwrap()).await {
            Ok(_) => {
                info!("Moved message {} to dead letter queue", msg_id);
                let _ = self.rsmq_queue.delete_message(&self.rsmq_queue_id, msg_id).await;
            }
            Err(e) => error!("Failed to move message {} to DLQ: {}", msg_id, e),
        }
    }

    async fn delete_proof(&self, job_id: &QProvingJobDataID) -> Result<()> {
        let proof_key = format!("proof:{}", job_id);
        let deleted: i32 = self.redis_pool.get().await?.del(&proof_key).await?;

        if deleted > 0 {
            info!("Deleted proof for job: {}", job_id);
        } else {
            warn!("Proof not found for job: {}", job_id);
        }
        Ok(())
    }

    async fn delete_witness(&self, job_id: &QProvingJobDataID) -> Result<()> {
        let witness_key = format!("witness:{}", job_id);
        let deleted: i32 = self.redis_pool.get().await?.del(&witness_key).await?;

        if deleted > 0 {
            info!("Deleted witness for job: {}", job_id);
        } else {
            warn!("Witness not found for job: {}", job_id);
        }
        Ok(())
    }

    async fn cleanup_job_data(&self, job_id: &QProvingJobDataID) -> Result<()> {
        let keys = [
            format!("job:proof:{}", job_id),
            format!("job:witness:{}", job_id),
            format!("job:metadata:{}", job_id),
            format!("job:timeout:{}", job_id),
        ];

        let deleted_count: i32 = redis::cmd("DEL")
            .arg(&keys)
            .query_async(&mut *self.redis_pool.get().await?)
            .await?;

        info!("Cleaned up {} keys for job: {}", deleted_count, job_id);
        Ok(())
    }

    async fn cleanup_job_status(&self, job_id: &QProvingJobDataID) -> Result<()> {
        info!("Cleaning up job {}", job_id);

        let keys = vec![
            QProvingTaskStoreImpl::job_status_key(job_id),
            QProvingTaskStoreImpl::job_timeout_key(job_id),
        ];

        let deleted_count: i32 = redis::cmd("DEL")
            .arg(&keys)
            .query_async(&mut *self.redis_pool.get().await?)
            .await?;

        if deleted_count > 0 {
            info!("Cleaned up job status for {:?} ({} keys deleted)", job_id, deleted_count);
        } else {
            info!("No job status found to clean up for {:?}", job_id);
        }

        Ok(())
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

async fn fetch_initial_block_height(node_type: &NodeType, store: &QEDStore) -> Result<u64> {

    let block_state = match node_type {
        NodeType::Coordinator => QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(store).await?,
        NodeType::Realm => QEDRealmStoreReaderAsync::get_latest_l2_block_state(store).await?,
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