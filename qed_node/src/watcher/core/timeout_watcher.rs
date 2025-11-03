use std::sync::Arc;

use anyhow::{anyhow, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use futures::StreamExt;
use qed_store::queue::task_queue::{current_timestamp_millis, QJobStatus, QProvingTaskStoreImpl, JOB_TIMEOUT_PREFIX};
use qed_store::queue::{QueueId, RsmqQueue};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use crate::watcher::constant::REDIS_LOCK_TTL_SECS;
use crate::watcher::events::{JobTimeoutEvent, WatcherMessage};

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub node_id: String,
    pub node_type: WatcherSourceNodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatcherSourceNodeType {
    Coordinator,
    Realm,
}

impl std::fmt::Display for WatcherSourceNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Coordinator => write!(f, "coordinator"),
            Self::Realm => write!(f, "realm"),
        }
    }
}

impl std::str::FromStr for WatcherSourceNodeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "coordinator" => Ok(Self::Coordinator),
            "realm" => Ok(Self::Realm),
            _ => Err(anyhow!("Invalid node type: {}", s)),
        }
    }
}

pub struct TimeoutWatcher {
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    redis_url: String,
    rsmq_queue: Arc<RsmqQueue>,
    queue_name: String,
    node_info: Arc<NodeInfo>,
    node_instance_id: String,
}

impl TimeoutWatcher {
    pub fn new(
        redis_pool: Arc<Pool<RedisConnectionManager>>,
        redis_url: String,
        rsmq_queue: Arc<RsmqQueue>,
        node_info: Arc<NodeInfo>,
        queue_name: &str,
    ) -> Self {
        Self {
            redis_pool,
            redis_url,
            rsmq_queue,
            queue_name: queue_name.to_string(),
            node_instance_id: format!("{}_{}", node_info.node_id, Uuid::new_v4()),
            node_info,
        }
    }

    pub async fn start_monitoring(&self) -> Result<()> {
        self.enable_keyspace_notifications().await?;

        let client = redis::Client::open(self.redis_url.clone())?;
        let mut pubsub = client.get_async_pubsub().await?;
        let channel = "__keyevent@0__:expired";

        pubsub.subscribe(channel).await?;
        debug!("Timeout watcher started for node {}, monitoring channel: {}",
              self.node_info.node_id, channel);

        let mut pubsub_stream = pubsub.into_on_message();

        while let Some(msg) = pubsub_stream.next().await {
            let key = match msg.get_payload::<String>() {
                Ok(k) => k,
                Err(e) => {
                    error!("Failed to get payload from Redis message: {}", e);
                    continue;
                }
            };

            if !key.starts_with(JOB_TIMEOUT_PREFIX) {
                continue;
            }

            match self.try_acquire_lock(&key).await {
                Ok(true) => {
                    if let Err(e) = self.handle_timeout_event(&key).await {
                        error!("Failed to handle timeout for {}: {}", key, e);
                    }
                }
                Ok(false) => {
                    debug!("Timeout event {} already being processed by another instance", key);
                }
                Err(e) => {
                    error!("Failed to acquire lock for {}: {}", key, e);
                }
            }
        }

        warn!("PubSub stream ended unexpectedly");
        Err(anyhow!("PubSub stream terminated"))
    }

    pub async fn check_redis_health(&self) -> Result<bool> {
        let mut conn = self.redis_pool.get().await?;
        let pong: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(pong == "PONG")
    }

    pub async fn stop(&self) -> Result<()> {
        debug!("Stopping timeout watcher for node {}", self.node_info.node_id);
        Ok(())
    }

    async fn try_acquire_lock(&self, timeout_key: &str) -> Result<bool> {
        let lock_key = format!("lock:timeout:{}", timeout_key);

        let result: Option<String> = redis::cmd("SET")
            .arg(&lock_key)
            .arg(&self.node_instance_id)
            .arg("NX")
            .arg("EX")
            .arg(REDIS_LOCK_TTL_SECS)
            .query_async(&mut *self.redis_pool.get().await?)
            .await?;

        Ok(result.is_some())
    }

    async fn enable_keyspace_notifications(&self) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;

        let config: Vec<String> = redis::cmd("CONFIG")
            .arg("GET")
            .arg("notify-keyspace-events")
            .query_async(&mut *conn)
            .await?;

        let current_config = config.get(1).map(String::as_str).unwrap_or("");

        if !current_config.contains('E') || !current_config.contains('x') {
            let new_config = format!("{}Ex", current_config);

            redis::cmd("CONFIG")
                .arg("SET")
                .arg("notify-keyspace-events")
                .arg(&new_config)
                .query_async::<()>(&mut *conn)
                .await?;

            debug!("Enabled Redis keyspace notifications: {}", new_config);
        } else {
            debug!("Redis keyspace notifications already enabled: {}", current_config);
        }

        Ok(())
    }

    async fn handle_timeout_event(&self, timeout_key: &str) -> Result<()> {
        let job_status_key = QProvingTaskStoreImpl::job_timeout_to_status(timeout_key)
            .ok_or_else(|| anyhow!("Failed to parse job status key from timeout key: {}", timeout_key))?;

        debug!("Job timeout detected: {}", job_status_key);

        let mut conn = self.redis_pool.get().await?;
        let status_bytes: Option<Vec<u8>> = conn.get(&job_status_key).await?;

        let job_status = status_bytes
            .ok_or_else(|| anyhow!("Job status not found for timed out job: {}", timeout_key))
            .and_then(|bytes| {
                QJobStatus::from_bytes(&bytes)
                    .map_err(|e| anyhow!("Failed to deserialize job status for {}: {}", timeout_key, e))
            })?;

        let timeout_event = JobTimeoutEvent {
            job_id: job_status.id.clone(),
            worker_id: job_status.worker_id.clone(),
            start_time: job_status.start_time,
            timeout_time: current_timestamp_millis(),
        };

        let message = WatcherMessage::JobTimeout(timeout_event.clone());
        let queue_id = QueueId::WorkerEvent {
            queue_biz_key: self.queue_name.clone(),
        };

        self.rsmq_queue
            .send_message(&queue_id, bincode::serialize(&message)?)
            .await
            .map_err(|e| {
                error!("Failed to push timeout event to queue: {}", e);
                anyhow::Error::from(e)
            })?;

        debug!(
            "Timeout event sent for job {} (worker: {:?}, started: {}, timed out: {})",
            job_status.id, job_status.worker_id, job_status.start_time, timeout_event.timeout_time
        );

        if let Err(e) = conn.del::<_, ()>(&job_status_key).await {
            warn!("Failed to clean up job status key {}: {}", job_status_key, e);
        }

        Ok(())
    }
}