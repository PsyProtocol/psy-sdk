use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use qed_store::queue::{QueueId, RsmqQueue};
use redis::AsyncCommands;
use rsmq::RsmqMessage;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};
use crate::common::utils::current_timestamp;
// use anyhow::Result;
use crate::watcher::ApiClient;
use crate::watcher::constant::{FAILURE_BACKOFF_DURATION, MAX_CONCURRENT_TASKS, MAX_RETRY_ATTEMPTS, MAX_SINGLE_MESSAGE_PROCESSING_TIME, RETRY_ATTEMPT_TTL_SECS, SLEEP_TIME_IF_HAVE_MSG, SLEEP_TIME_IF_NO_MSG};
use crate::watcher::error::WatcherError;
use crate::watcher::events::WatcherMessage;

pub struct MessageProcessor {
    rsmq_queue: Arc<RsmqQueue>,
    queue_id: QueueId,
    api_client: Arc<ApiClient>,
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    node_id: String,
}

impl MessageProcessor {
    pub fn new(
        rsmq_queue: Arc<RsmqQueue>,
        queue_id: QueueId,
        api_client: Arc<ApiClient>,
        redis_pool: Arc<Pool<RedisConnectionManager>>,
        node_id: String,
    ) -> Self {
        Self {
            rsmq_queue,
            queue_id,
            api_client,
            redis_pool,
            node_id,
        }
    }

    pub async fn run(&self) -> Result<(), WatcherError> {
        info!("Starting message processor");

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));
        let active_tasks = Arc::new(AtomicUsize::new(0));

        loop {
            let current_active = active_tasks.load(Ordering::Relaxed);

            if self.is_at_capacity(current_active).await {
                continue;
            }

            match self.receive_next_message().await {
                Ok(Some(msg)) => {
                    self.spawn_message_handler(msg, &semaphore, &active_tasks).await?;
                }
                Ok(None) => {
                    self.wait_for_messages(current_active).await;
                }
                Err(e) => {
                    error!("Failed to receive message: {}", e);
                    tokio::time::sleep(FAILURE_BACKOFF_DURATION).await;
                }
            }
        }
    }

    async fn is_at_capacity(&self, current_active: usize) -> bool {
        if current_active < MAX_CONCURRENT_TASKS {
            return false;
        }

        debug!("At max capacity ({} tasks), waiting...", current_active);
        tokio::time::sleep(SLEEP_TIME_IF_HAVE_MSG).await;
        true
    }

    async fn receive_next_message(&self) -> Result<Option<RsmqMessage<Vec<u8>>>, WatcherError> {
        self.rsmq_queue
            .receive_message_with_id(&self.queue_id, Some(MAX_SINGLE_MESSAGE_PROCESSING_TIME))
            .await
            .map_err(|e| WatcherError::Queue(e.to_string()))
    }

    async fn wait_for_messages(&self, current_active: usize) {
        let delay = if current_active == 0 {
            SLEEP_TIME_IF_NO_MSG
        } else {
            SLEEP_TIME_IF_HAVE_MSG
        };
        tokio::time::sleep(delay).await;
    }

    async fn spawn_message_handler(
        &self,
        msg: RsmqMessage<Vec<u8>>,
        semaphore: &Arc<Semaphore>,
        active_tasks: &Arc<AtomicUsize>,
    ) -> Result<(), WatcherError> {
        let permit = semaphore.clone().acquire_owned().await
            .map_err(|e| WatcherError::Queue(format!("Failed to acquire semaphore: {}", e)))?;

        let processor = MessageProcessorHandle {
            rsmq_queue: self.rsmq_queue.clone(),
            queue_id: self.queue_id.clone(),
            api_client: self.api_client.clone(),
            redis_pool: self.redis_pool.clone(),
            node_id: self.node_id.clone(),
        };

        let active_tasks_clone = active_tasks.clone();
        active_tasks.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            let _permit = permit;
            processor.process_message(msg).await;
            active_tasks_clone.fetch_sub(1, Ordering::Relaxed);
        });

        Ok(())
    }
}

struct MessageProcessorHandle {
    rsmq_queue: Arc<RsmqQueue>,
    queue_id: QueueId,
    api_client: Arc<ApiClient>,
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    node_id: String,
}

impl MessageProcessorHandle {
    async fn process_message(&self, msg: RsmqMessage<Vec<u8>>) {
        let msg_id = &msg.id;

        let message = match bincode::deserialize::<WatcherMessage>(&msg.message) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to deserialize message {}: {}", msg_id, e);
                self.delete_message(msg_id).await;
                return;
            }
        };

        debug!("Processing message {}", msg_id);

        let attempts = self.get_message_attempts(msg_id).await.unwrap_or(0);

        if let Err(e) = self.handle_message(&message).await {
            self.handle_processing_failure(msg_id, &message, attempts, e).await;
            return;
        }

        self.complete_message(msg_id).await;
    }

    async fn handle_message(&self, message: &WatcherMessage) -> Result<(), WatcherError> {
        use WatcherMessage::*;

        match message {
            UserRegistration(event) => {
                info!("UserEvent: registration pk={}", event.metadata.public_key);
                self.api_client.send_user_registration(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
            DeployContract(event) => {
                info!("UserEvent: contract deployment deployer={}", event.deployer);
                self.api_client.send_contract_deployment(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
            GutaSubmission(event) => {
                info!("UserEvent: GUTA realm={} circuit={}",
                    event.realm_id, event.metadata.circuit_type);
                self.api_client.send_guta_submission(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
            EndcapSubmission(event) => {
                info!("UserEvent: Endcap realm={} user={}", event.realm_id, event.user_id);
                self.api_client.send_endcap_submission(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
            JobPending(event) => {
                info!("JobEvent: pending {:?}", event.job_id);
                self.api_client.send_job_pending(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
            JobStarted(event) => {
                info!("JobEvent: started {:?} worker={}", event.job_id, event.worker_id);
                self.api_client.send_job_started(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
            JobCompleted(event) => {
                info!("JobEvent: completed {:?} worker={:?}", event.job_id, event.worker_id);
                self.api_client.send_job_completed(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
            JobTimeout(event) => {
                warn!("JobEvent: timeout {:?}", event.job_id);
                self.api_client.send_job_timeout(event.clone()).await
                    .map_err(|e| WatcherError::ApiClient(e.to_string()))
            }
        }
    }

    async fn complete_message(&self, msg_id: &str) {
        if let Err(e) = self.rsmq_queue.delete_message(&self.queue_id, msg_id).await {
            error!("Failed to delete message {}: {}", msg_id, e);
            return;
        }

        debug!("Successfully processed message {}", msg_id);
        self.clear_redis_key(&format!("watcher:msg_attempts:{}", msg_id)).await;
    }

    async fn handle_processing_failure(
        &self,
        msg_id: &str,
        message: &WatcherMessage,
        attempts: u32,
        error: WatcherError,
    ) {
        let attempt_count = attempts + 1;
        error!("Failed to process message {} (attempt {}): {}", msg_id, attempt_count, error);

        if attempt_count >= MAX_RETRY_ATTEMPTS {
            self.send_to_dead_letter_queue(msg_id, message, error).await;
            return;
        }

        self.increment_message_attempts(msg_id).await;
    }

    async fn send_to_dead_letter_queue(
        &self,
        msg_id: &str,
        message: &WatcherMessage,
        error: WatcherError,
    ) {
        warn!("Message {} failed {} times, moving to DLQ", msg_id, MAX_RETRY_ATTEMPTS);

        let dlq_id = QueueId::WorkerEvent {
            queue_biz_key: format!("{}_dlq", self.queue_id.get_queue_id()),
        };

        let _ = self.rsmq_queue.create_queue_if_not_exists(&dlq_id).await;

        let dlq_message = serde_json::json!({
            "original_msg_id": msg_id,
            "message": message,
            "error": error.to_string(),
            "failed_at": current_timestamp(),
            "node_id": self.node_id,
        });

        if let Ok(serialized) = serde_json::to_vec(&dlq_message) {
            if let Ok(_) = self.rsmq_queue.send_message(&dlq_id, serialized).await {
                info!("Moved message {} to DLQ", msg_id);
                self.delete_message(msg_id).await;
            }
        }

        self.clear_redis_key(&format!("watcher:msg_attempts:{}", msg_id)).await;
    }

    async fn delete_message(&self, msg_id: &str) {
        let _ = self.rsmq_queue.delete_message(&self.queue_id, msg_id).await;
    }

    async fn get_message_attempts(&self, msg_id: &str) -> Option<u32> {
        let key = format!("watcher:msg_attempts:{}", msg_id);
        self.redis_pool.get().await.ok()?
            .get(&key).await.ok()
    }

    async fn increment_message_attempts(&self, msg_id: &str) {
        let key = format!("watcher:msg_attempts:{}", msg_id);

        if let Ok(mut conn) = self.redis_pool.get().await {
            let attempts = conn.get::<_, u32>(&key).await.unwrap_or(0) + 1;
            let _: std::result::Result<(), redis::RedisError> =
                conn.set_ex(&key, attempts, RETRY_ATTEMPT_TTL_SECS).await;
        }
    }

    async fn clear_redis_key(&self, key: &str) {
        if let Ok(mut conn) = self.redis_pool.get().await {
            let _: std::result::Result<i32, redis::RedisError> = conn.del(key).await;
        }
    }
}