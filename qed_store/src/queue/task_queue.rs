use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH,}
};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use qed_core::{
    job::id::{LayerId, QProvingJobDataID, QProvingJobGraph, QProvingTask, QProvingTaskGraph, QProvingTaskLayer, TaskId},
    utils::graph::BidirectionalGraph,
};
use redis::AsyncCommands;
use scylla::_macro_internal::SerializeRow;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};

use crate::queue::{new_redis_async_pool, QueueId, QueueStats, RsmqQueue};

const TASK_COMMON_PREFIX: &str = "tasks:";
pub const JOB_STATUS_PREFIX: &str = "job-status:";
pub const JOB_TIMEOUT_PREFIX: &str = "job-timeout:";
const JOB_TIMEOUT_SECONDS: u64 =  6;
const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(JOB_TIMEOUT_SECONDS);

/// Represents a single proving job with task assignment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QJob<T = String> {
    pub job_id: QProvingJobDataID,
    pub layer_id: LayerId,
    pub parent: Option<QProvingJobDataID>,
    #[serde(default)]
    pub msg_id: T,
}

impl<T: Default> QJob<T> {
    pub fn new(job_id: QProvingJobDataID, layer_id: LayerId) -> Self {
        Self {
            job_id,
            layer_id,
            parent: None,
            msg_id: T::default(),
        }
    }

    pub fn new_with_parent(job_id: QProvingJobDataID, layer_id: LayerId, parent: QProvingJobDataID) -> Self {
        Self {
            job_id,
            layer_id: layer_id,
            parent: Some(parent),
            msg_id: T::default(),
        }
    }
}

impl QJob<String> {
    pub fn with_msg_id(mut self, msg_id: String) -> Self {
        self.msg_id = msg_id;
        self
    }

    /// Serialize to bytes using bincode
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Failed to serialize QJob")
    }

    /// Deserialize from bytes using bincode
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Failed to deserialize QJob")
    }

    /// Check if this job has been assigned a message ID
    pub fn has_msg_id(&self) -> bool {
        !self.msg_id.is_empty()
    }
}

// Display implementation for better logging
impl std::fmt::Display for QJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Job({:?} layer:{})", self.job_id, self.layer_id)
    }
}
/// Job status structure integrated into the task store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QJobStatus {
    pub id: QProvingJobDataID,
    pub status: Status,
    pub worker_id: Option<String>,
    pub start_time: u64, // Milliseconds since epoch
    pub end_time: Option<u64>, // Milliseconds since epoch
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Status {
    Processing,
    Completed,
}

impl QJobStatus {
    /// Create a new job status for a claimed job
    pub fn new_processing(id: QProvingJobDataID, worker_id: &str) -> Self {
        Self {
            id,
            status: Status::Processing,
            worker_id: Some(worker_id.to_string()),
            start_time: current_timestamp_millis(),
            end_time: None,
        }
    }

    /// Mark job as completed
    pub fn mark_completed(&mut self) {
        self.status = Status::Completed;
        self.end_time = Some(current_timestamp_millis());
    }

    /// Reset job status when reclaimed by another worker
    pub fn reset_for_reclaim(&mut self, new_worker_id: &str) {
        self.status = Status::Processing;
        self.worker_id = Some(new_worker_id.to_string());
        self.start_time = current_timestamp_millis();
        self.end_time = None;
    }

    /// Serialize to bytes using bincode
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Failed to serialize QJobStatus")
    }

    /// Deserialize from bytes using bincode
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Failed to deserialize QJobStatus")
    }

    /// Calculate duration in milliseconds (only for completed jobs)
    pub fn duration_ms(&self) -> Option<u64> {
        match (self.status.clone(), self.end_time) {
            (Status::Completed, Some(end)) => Some(end - self.start_time),
            _ => None,
        }
    }

    /// Check if the job has timed out (based on current time and timeout duration)
    pub fn is_timed_out(&self, timeout_ms: u64) -> bool {
        if self.status == Status::Completed {
            return false;
        }
        let now = current_timestamp_millis();
        (now - self.start_time) > timeout_ms
    }
}

/// Job task store implementation with Redis backend and layer support
#[derive(Clone)]
pub struct QProvingTaskStoreImpl {
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    rsmq: Arc<RsmqQueue>,
    task_graph: Arc<Mutex<QProvingTaskGraph>>,
    job_graph: Arc<Mutex<QProvingJobGraph>>,
}

impl QProvingTaskStoreImpl {
    pub async fn new(redis_url: &str, pool_size: usize) -> Result<Self> {
        debug!("Initializing JobTaskStore with pool size {}", pool_size);

        let redis_pool = Arc::new(new_redis_async_pool(redis_url, pool_size).await.context("Failed to create Redis pool")?);

        let rsmq = Arc::new(
            RsmqQueue::new(redis_url, pool_size, "task_store")
                .await
                .context("Failed to create RSMQ queue")?,
        );

        Ok(Self {
            redis_pool,
            rsmq,
            task_graph: Arc::new(Mutex::new(QProvingTaskGraph::new())),
            job_graph: Arc::new(Mutex::new(QProvingJobGraph::new())),
        })
    }

    /// Get the checkpoint-specific graph key
    #[inline]
    fn graph_key(&self, checkpoint_id: u64) -> String {
        format!("{}:{}:graph", TASK_COMMON_PREFIX, checkpoint_id)
    }

    /// Get the job graph history key for LRU cache management
    #[inline]
    fn job_graph_history_key(&self) -> String {
        format!("{}:job_graph_history", TASK_COMMON_PREFIX)
    }

    /// Get the checkpoint-specific layers key
    #[inline]
    fn layers_key(&self) -> String {
        format!("{}:layers_zset", TASK_COMMON_PREFIX)
    }

    /// Generate queue name for a layer (includes checkpoint)
    #[inline]
    fn layer_queue_name(&self, layer_id: &LayerId) -> String {
        format!("{}:{}:rsmq", TASK_COMMON_PREFIX, layer_id)
    }

    /// Generate job-status key
    #[inline]
    pub fn job_status_key(job_id: &QProvingJobDataID) -> String {
        format!("{}:{}", JOB_STATUS_PREFIX, job_id.to_hex_string())
    }

    /// Generate job-timeout key
    #[inline]
    pub fn job_timeout_key(job_id: &QProvingJobDataID) -> String {
        format!("{}:{}", JOB_TIMEOUT_PREFIX, job_id.to_hex_string())
    }

    #[inline]
    pub fn job_timeout_to_status(timeout_key: &str) -> Option<String> {
        if let Some(rest) = timeout_key.strip_prefix(JOB_TIMEOUT_PREFIX) {
            Some(format!("{}:{}", JOB_STATUS_PREFIX, rest))
        } else {
            None
        }
    }

    #[inline]
    pub fn job_status_to_timeout(status_key: &str) -> Option<String> {
        if let Some(rest) = status_key.strip_prefix(JOB_STATUS_PREFIX) {
            Some(format!("{}:{}", JOB_TIMEOUT_PREFIX, rest))
        } else {
            None
        }
    }

    /// Set job status in Redis
    async fn set_job_status(&self, status: &QJobStatus) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key =  Self::job_status_key(&status.id);
        let value = status.to_bytes()?;
        debug!("Setting job status in Redis: key={}, status={:?}", key, status);
        conn.set(key, value).await?;
        Ok(())
    }

    /// Get job status from Redis
    pub async fn get_job_status(&self, job_id: &QProvingJobDataID) -> Result<Option<QJobStatus>> {
        let mut conn = self.redis_pool.get().await?;
        let key =  Self::job_status_key(job_id);
        let value: Option<Vec<u8>> = conn.get(key).await?;
        match value {
            Some(bytes) => Ok(Some(QJobStatus::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Delete job status from Redis
    async fn delete_job_status(&self, job_id: &QProvingJobDataID) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key =  Self::job_status_key(job_id);
        conn.del(key).await?;
        Ok(())
    }

    /// Set job timeout key with expiration
    async fn set_job_timeout(&self, job_id: &QProvingJobDataID, timeout_seconds: u64) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key =  Self::job_timeout_key(job_id);
        // Set empty value with expiration
        conn.set_ex(key, "", timeout_seconds).await?;
        Ok(())
    }

    /// Delete job timeout key
    async fn delete_job_timeout(&self, job_id: &QProvingJobDataID) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key = Self::job_timeout_key(job_id);
        conn.del(key).await?;
        Ok(())
    }

    //todo! maybe use redis ttl mechanism for job status cleanup
    /// Schedule job status cleanup with TTL
    async fn schedule_status_cleanup(&self, job_id: &QProvingJobDataID) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key =  Self::job_status_key(job_id);

        // Set TTL to 1 hour (3600 seconds) for cleanup
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(3600)
            .query_async(&mut *conn)
            .await?;

        info!("Scheduled cleanup for job-status: {}", job_id);
        Ok(())
    }

    /// Clean up stale job statuses (can be called by watcher or maintenance task)
    pub async fn cleanup_completed_job_statuses(&self, before_timestamp: u64) -> Result<usize> {
        let mut conn = self.redis_pool.get().await?;

        // Get all job-status keys
        let pattern = "job-status:*";
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await?;

        let mut cleaned = 0;
        for key in keys {
            // Extract job ID from key (skip "job-status:" prefix)
            if let Some(job_id_str) = key.strip_prefix("job-status:") {
                if let Ok(job_id) = QProvingJobDataID::try_from_byte_vec(&hex::decode(job_id_str)?) {
                    if let Ok(Some(status)) = self.get_job_status(&job_id).await {
                        // Check if job is completed and old enough
                        if status.status == Status::Completed {
                            if let Some(end_time) = status.end_time {
                                if end_time < before_timestamp {
                                    self.delete_job_status(&job_id).await?;
                                    cleaned += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(cleaned)
    }

    /// Get all active jobs (Processing status)
    pub async fn get_active_jobs(&self) -> Result<Vec<QJobStatus>> {
        let mut conn = self.redis_pool.get().await?;

        // Get all job-status keys
        let pattern = "job-status:*";
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await?;

        let mut active_jobs = Vec::new();
        for key in keys {
            if let Some(job_id_str) = key.strip_prefix("job-status:") {
                if let Ok(job_id) = QProvingJobDataID::try_from_byte_vec(&hex::decode(job_id_str)?) {
                    if let Ok(Some(status)) = self.get_job_status(&job_id).await {
                        if status.status == Status::Processing {
                            active_jobs.push(status);
                        }
                    }
                }
            }
        }

        Ok(active_jobs)
    }

    /// Get job statistics
    pub async fn get_job_statistics(&self) -> Result<JobStatistics> {
        let mut conn = self.redis_pool.get().await?;

        // Get all job-status keys
        let pattern = "job-status:*";
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await?;

        let mut stats = JobStatistics::default();

        for key in keys {
            if let Some(job_id_str) = key.strip_prefix("job-status:") {
                if let Ok(job_id) = QProvingJobDataID::try_from_byte_vec(&hex::decode(job_id_str)?) {
                    if let Ok(Some(status)) = self.get_job_status(&job_id).await {
                        match status.status {
                            Status::Processing => {
                                stats.processing_count += 1;
                                let duration = current_timestamp_millis() - status.start_time;
                                stats.avg_processing_time =
                                    (stats.avg_processing_time * (stats.processing_count - 1) as u64 + duration)
                                        / stats.processing_count as u64;
                            }
                            Status::Completed => {
                                stats.completed_count += 1;
                                if let Some(duration) = status.duration_ms() {
                                    stats.avg_completion_time =
                                        (stats.avg_completion_time * (stats.completed_count - 1) as u64 + duration)
                                            / stats.completed_count as u64;
                                }
                            }
                        }
                    }
                }
            }
        }

        stats.total_count = stats.processing_count + stats.completed_count;
        Ok(stats)
    }
    /// Create QueueId for a layer
    #[inline]
    fn layer_queue_id(&self, layer_id: &LayerId) -> QueueId {
        QueueId::WorkerEvent {
            queue_biz_key: self.layer_queue_name(layer_id),
        }
    }

    /// Push layers to the tail of the list
    async fn push_layers(&self, layers: &[QProvingTaskLayer]) -> Result<()> {
        if layers.is_empty() {
            return Ok(());
        }

        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        // Use timestamp + sequence for unique, ordered scores
        let base_score = chrono::Utc::now().timestamp_millis() as f64;

        // Use pipeline for efficiency
        let mut pipe = redis::pipe();
        pipe.atomic();

        for (idx, layer) in layers.iter().enumerate() {
            let serialized = bincode::serialize(&layer.layer_id).context("push_layers: Failed to serialize layer id")?;
            // Score ensures ordering: earlier layers have lower scores
            let score = base_score + (idx as f64);
            pipe.zadd(&layers_key, serialized, score);
        }

        pipe.query_async(&mut *conn).await?;

        info!("Pushed {} layers to sorted set", layers.len()); // Changed from "list"
        Ok(())
    }

    /// Peek at the current layer (head of list) without removing
    async fn peek_current_layer(&self) -> Result<Option<LayerId>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        // Get the first element without removing it
        // ZRANGE gets elements by rank (0 = lowest score = first layer)
        let result: Vec<Vec<u8>> = redis::cmd("ZRANGE")
            .arg(&layers_key)
            .arg(0) // start rank
            .arg(0) // end rank (just get first element)
            .query_async(&mut *conn)
            .await?;

        match result.first() {
            Some(data) => {
                let layer = bincode::deserialize(data)?;
                Ok(Some(layer))
            }
            None => Ok(None),
        }
    }

    /// Atomically pop the top layer only if it matches the expected layer ID
    async fn pop_current_layer(&self, expected_layer_id: &LayerId) -> Result<Option<LayerId>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        // Check if this layer is actually at position 0
        let rank = self.get_layer_rank(expected_layer_id).await?;

        match rank {
            Some(0) => {
                // It's the first element, safe to remove
                let expected_serialized = bincode::serialize(expected_layer_id).context("pop_current_layer: Failed to serialize layer id")?;
                let removed: i32 = conn.zrem(&layers_key, &expected_serialized).await?;

                if removed > 0 {
                    trace!("Successfully popped layer {:?} from head", expected_layer_id);
                    Ok(Some(*expected_layer_id))
                } else {
                    // Shouldn't happen, but handle gracefully
                    warn!("Layer was at rank 0 but couldn't be removed");
                    Ok(None)
                }
            }
            Some(n) => {
                warn!("⚠️ Layer {:?} is at position {}, not at head", expected_layer_id, n);
                Ok(None)
            }
            None => {
                info!("Layer {:?} not found in sorted set", expected_layer_id);
                Ok(None)
            }
        }
    }

    /// Get the total number of remaining layers
    async fn get_layer_count(&self) -> Result<usize> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();
        let count: usize = conn.zcard(&layers_key).await?;
        Ok(count)
    }

    /// Get all layers without removing them (for monitoring)
    async fn get_all_layers(&self) -> Result<Vec<LayerId>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        // FIX: Use ZRANGE for sorted sets, not lrange
        let all_bytes: Vec<Vec<u8>> = redis::cmd("ZRANGE")
            .arg(&layers_key)
            .arg(0) // start index
            .arg(-1) // end index (all elements)
            .query_async(&mut *conn)
            .await?;

        let mut layers = Vec::with_capacity(all_bytes.len());
        for bytes in all_bytes {
            layers.push(bincode::deserialize(&bytes)?);
        }

        Ok(layers)
    }

    /// Check if a layer is complete
    async fn is_layer_complete(&self, layer_id: &LayerId) -> Result<bool> {
        let queue_id = self.layer_queue_id(layer_id);
        let stats = self.rsmq.get_queue_stats(&queue_id).await?;

        // Layer is only complete when BOTH visible and hidden messages are 0
        let is_complete = stats.total_messages == 0 && stats.hidden_messages == 0;

        if !is_complete && stats.total_messages == 0 {
            // Log when we have only hidden messages (jobs being processed)
            warn!(
                "❗ Layer {} has {} hidden messages still being processed",
                layer_id, stats.hidden_messages
            );
        }
        Ok(is_complete)
    }

    /// Get queue statistics for monitoring
    pub async fn get_queue_stats(&self) -> Result<HashMap<LayerId, u64>> {
        let layers = self.get_all_layers().await?;
        let mut stats = HashMap::with_capacity(layers.len());

        for layer in layers {
            let queue_id = self.layer_queue_id(&layer);
            if let Ok(count) = self.rsmq.get_queue_length(&queue_id).await {
                stats.insert(layer, count);
            }
        }

        Ok(stats)
    }

    /// Get system status summary
    pub async fn get_system_status(&self) -> Result<String> {
        let layers = self.get_all_layers().await?;
        let current_layer = self.peek_current_layer().await?;
        let stats = self.get_queue_stats().await?;

        let mut status = format!("Total Layers: {}\n", layers.len());

        if let Some(current) = current_layer {
            status.push_str(&format!("Current Layer: {}\n", current));
        } else {
            status.push_str("Current Layer: None (all completed)\n");
        }

        for layer in layers {
            let count = stats.get(&layer).copied().unwrap_or(0);
            status.push_str(&format!("  Layer {}: {} pending jobs\n", layer, count));
        }

        Ok(status)
    }
}

impl QProvingTaskStoreImpl {
    /// Validate that a specific job is currently being processed by a worker
    /// Returns a detailed status of the validation
    pub async fn validate_job_ownership(&self, job: &QJob) -> Result<JobValidationStatus> {
        // Step 1: Check if the layer is the current layer
        let current_layer = match self.peek_current_layer().await? {
            Some(layer) => layer,
            None => {
                warn!("No active layer when validating job {}", job);
                return Ok(JobValidationStatus::NoActiveLayer);
            }
        };

        if current_layer != job.layer_id {
            warn!("Job {} claims layer {} but current layer is {}", job, job.layer_id, current_layer);
            return Ok(JobValidationStatus::WrongLayer {
                expected: current_layer,
                provided: job.layer_id,
            });
        }

        // Step 2: Trying to change message visibility, extending the visibility timeout
        let queue_id = self.layer_queue_id(&job.layer_id);

        match self.rsmq.change_message_visibility(&queue_id, &job.msg_id, VISIBILITY_TIMEOUT).await {
            Ok(_) => {
                debug!("Job {} validated: message {} is hidden and visibility extended", job, job.msg_id);
                Ok(JobValidationStatus::Valid)
            }
            Err(e) => {
                // Failed - analyze the error to determine why
                let error_str = e.to_string().to_lowercase();

                if error_str.contains("not found") || error_str.contains("does not exist") {
                    warn!("Job {} validation failed: message {} not found in queue", job, job.msg_id);
                    Ok(JobValidationStatus::MessageNotFound)
                } else if error_str.contains("visible") || error_str.contains("not hidden") {
                    warn!("Job {} validation failed: message {} is visible (not being processed)", job, job.msg_id);
                    Ok(JobValidationStatus::MessageNotHidden)
                } else {
                    // Unexpected error - propagate it
                    error!("Unexpected error validating job {}: {}", job, e);
                    Err(e)
                }
            }
        }
    }

    /// Simplified validation that just returns true/false
    pub async fn is_job_valid(&self, job: &QJob) -> Result<bool> {
        match self.validate_job_ownership(job).await? {
            JobValidationStatus::Valid => Ok(true),
            _ => Ok(false),
        }
    }

    pub async fn make_job_visible_again(&self, job: &QJob) -> Result<()> {
        let queue_id = self.layer_queue_id(&job.layer_id);
        self.rsmq.change_message_visibility(&queue_id, &job.msg_id, Duration::from_secs(0)).await?;
        Ok(())
    }

    /// Set custom visibility timeout for a job
    /// This allows you to control when the job becomes available for other workers to claim
    pub async fn set_job_visibility(&self, job: &QJob, visibility_seconds: u64) -> Result<()> {
        let queue_id = self.layer_queue_id(&job.layer_id);
        let visibility = Duration::from_secs(visibility_seconds);

        self.rsmq
            .change_message_visibility(&queue_id, &job.msg_id, visibility)
            .await
            .context(format!(
                "Failed to set visibility to {} seconds for job {}",
                visibility_seconds, job
            ))?;

        info!(
            "Set visibility to {} seconds for job {} (msg_id: {})",
            visibility_seconds, job, job.msg_id
        );

        Ok(())
    }


}

#[derive(Debug, PartialEq, Clone)]
pub enum JobValidationStatus {
    Valid,
    NoActiveLayer,
    WrongLayer { expected: LayerId, provided: LayerId },
    MessageNotFound,
    MessageNotHidden,
}
/// Job statistics for monitoring
#[derive(Debug, Default, Clone)]
pub struct JobStatistics {
    pub total_count: usize,
    pub processing_count: usize,
    pub completed_count: usize,
    pub avg_processing_time: u64,  // milliseconds
    pub avg_completion_time: u64,  // milliseconds
}


#[async_trait]
pub trait QProvingTaskStore {
    async fn save_task_topology_with_layers(&self, graph: Vec<QProvingTaskLayer>) -> Result<()>;
    async fn claim_job_from_current_layer(&self, worker_id: &str) -> Result<Option<QJob>>;
    async fn acknowledge_job_completion(&self, job: &QJob, worker_id: &str) -> Result<QJobStatus>;
    async fn get_current_layer_info(&self) -> Result<Option<LayerId>>;
    async fn count_pending_jobs_in_current_layer(&self) -> Result<u64>;

    // Task graph management methods
    async fn write_next_tasks(&self, task: &QProvingTask, next_task: &QProvingTask) -> Result<()>;
    async fn write_multidimensional_tasks(&self, tasks: &[QProvingTask], next_task: &QProvingTask) -> Result<()>;
    async fn add_task(&self, task: &QProvingTask) -> Result<()>;
    async fn get_task_graph(&self) -> QProvingTaskGraph;
    async fn clear_task_graph(&self) -> Result<()>;
    async fn finalize_and_save_topology(&self) -> Result<()>;

    async fn set_job_dependency_graph(
        &self,
        deploy_contracts_graph: BidirectionalGraph<QProvingJobDataID>,
        user_registrations_graph: BidirectionalGraph<QProvingJobDataID>,
        guta_graph: BidirectionalGraph<QProvingJobDataID>,
    ) -> Result<()>;

    async fn get_graph_for_job(&self, job_id: &QProvingJobDataID) -> Result<BidirectionalGraph<QProvingJobDataID>>;

    // Legacy operations (kept for compatibility)
    async fn save_job_dependency_graph(&self, checkpoint_id: u64) -> Result<()>;

    async fn clear_job_dependency_graph(&self, checkpoint_id: u64) -> Result<()>;
    async fn load_job_dependency_graph(&self, checkpoint_id: u64) -> Result<QProvingJobGraph>;
}

#[async_trait]
impl QProvingTaskStore for QProvingTaskStoreImpl {
    async fn save_task_topology_with_layers(&self, layers: Vec<QProvingTaskLayer>) -> Result<()> {
        info!("Saving task topology with layer support");

        // Step 1: Create an RSMQ queue for each layer and send jobs
        for layer in &layers {
            // Create queue for this layer
            let queue_id = self.layer_queue_id(&layer.layer_id);
            self.rsmq.create_queue_if_not_exists(&queue_id).await?;

            // Create QJob instances for all job IDs in this layer
            let jobs: Vec<QJob> = layer.job_ids.iter().map(|job_id| QJob::new(job_id.clone(), layer.layer_id)).collect();

            // Send all jobs to the corresponding layer queue
            if !jobs.is_empty() {
                self.rsmq.send_batch(&queue_id, &jobs).await?;
                trace!("Sent {} jobs to layer {} queue", jobs.len(), layer.layer_id);
            }
        }

        // Step 2: Clear existing layers list for this checkpoint
        let mut conn = self.redis_pool.get().await?;
        conn.del(&self.layers_key()).await?;

        // Step 3: Send all layers to the Redis list
        self.push_layers(&layers).await?;
        trace!("Successfully saved {} layers", layers.len());
        Ok(())
    }

    async fn claim_job_from_current_layer(&self, worker_id: &str) -> Result<Option<QJob>> {

        // Peek at the current layer (head of the list)
        let current_layer = match self.peek_current_layer().await? {
            Some(layer) => layer,
            None => {
                trace!("No layers available");
                return Ok(None);
            }
        };

        let queue_id = self.layer_queue_id(&current_layer);

        // Try to claim a job from the current layer
        match self.rsmq.receive_object_with_id::<QJob>(&queue_id, Some(VISIBILITY_TIMEOUT)).await? {
            Some((mut job, msg_id)) => {
                job = job.with_msg_id(msg_id);

                // Check if this job was previously claimed (reclaim scenario)
                let existing_status = self.get_job_status(&job.job_id).await?;

                let job_status = if let Some(mut status) = existing_status {
                    // Job was previously claimed, reset for reclaim
                    info!("Reclaiming job {} (was worker: {:?})", job.job_id, status.worker_id);
                    status.reset_for_reclaim(worker_id);
                    status
                } else {
                    // New job claim
                    QJobStatus::new_processing(job.job_id.clone(), worker_id)
                };

                // Set job-status with processing state
                self.set_job_status(&job_status).await?;

                // Set job-timeout with 30 seconds expiration
                self.set_job_timeout(&job.job_id, JOB_TIMEOUT_SECONDS).await?;

                info!("Claimed {} from layer {} with status tracking", job, current_layer);
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    async fn acknowledge_job_completion(&self, job: &QJob, worker_id: &str) -> Result<QJobStatus> {
        trace!("Acknowledging job completion  {}", job);

        let queue_id = self.layer_queue_id(&job.layer_id);

        // Get current job status
        let mut job_status = self.get_job_status(&job.job_id).await?
            .ok_or_else(|| anyhow!("Job status not found for {}", job.job_id))?;

        // CRITICAL: Verify that the worker attempting to complete the job is the one who claimed it
        match &job_status.worker_id {
            Some(claimed_worker_id) if claimed_worker_id == worker_id => {
                // Worker ID matches - proceed with completion
                info!("Worker ID verified for job {}", job.job_id);
            }
            Some(claimed_worker_id) => {
                // Different worker trying to complete the job - this is an error
                error!(
                    "Worker {} attempted to complete job {} which was claimed by worker {}",
                    worker_id, job.job_id, claimed_worker_id
                );
                return Err(anyhow!(
                    "Worker ID mismatch: job {} was claimed by {} but {} attempted to complete it",
                    job.job_id, claimed_worker_id, worker_id
                ));
            }
            None => {
                // in fact, this should never happen for a processing job
                // but let's handle it gracefully
                // No worker ID in status - this shouldn't happen for a processing job
                error!("Job {} has no worker ID in status", job.job_id);
                return Err(anyhow!(
                    "Job {} has no worker ID - cannot verify completion authority",
                    job.job_id
                ));
            }
        }

        // Verify job is in Processing state (not already completed)
        if job_status.status != Status::Processing {
            warn!(
                "Attempted to complete job {} with status {:?} by worker {}",
                job.job_id, job_status.status, worker_id
            );
            return Err(anyhow!(
                "Job {} is not in Processing state (current: {:?})",
                job.job_id, job_status.status
            ));
        }

        // Mark job as completed
        job_status.mark_completed();
        self.set_job_status(&job_status).await?;

        // Delete job-timeout key to prevent timeout event
        self.delete_job_timeout(&job.job_id).await?;

        // Delete message from queue,
        // note: it should after the proof has been verified
        self.rsmq.delete_message(&queue_id, &job.msg_id).await?;

        // Check if the layer is complete
        let remaining = self.rsmq.get_queue_length(&queue_id).await?;

        if remaining == 0 {
            trace!("Layer {} has no more jobs", job.layer_id);

            // Schedule job-status cleanup
            //todo! maybe use redis ttl mechanism for job status cleanup
            self.schedule_status_cleanup(&job.job_id).await?;

            // Check if this is the current layer and remove it if complete
            if let Some(current_layer) = self.peek_current_layer().await? {
                if current_layer == job.layer_id && self.is_layer_complete(&job.layer_id).await? {
                    match self.pop_current_layer(&job.layer_id).await {
                        Ok(s) => {
                            match s {
                                Some(popped_layer) => {
                                    // Actually popped the layer
                                    trace!("Successfully removed completed layer {} from list", popped_layer);

                                    if let Some(next_layer) = self.peek_current_layer().await? {
                                        trace!("Next layer is {}", next_layer);
                                    } else {
                                        info!("All layers completed!");
                                    }
                                }
                                None => {
                                    // pop_current_layer returned Ok(None) - layer wasn't at top or didn't match
                                    warn!(
                                        "Layer {} is complete but couldn't be popped (not at top or already removed by another thread)",
                                        job.layer_id
                                    );

                                    // Debug: Check what's actually at the top
                                    if let Some(actual_top) = self.peek_current_layer().await? {
                                        warn!("Current top layer is actually: {}", actual_top);
                                    }

                                    // Print all layers for debugging
                                    info!("⚠️ Layer pop returned None, debugging layers:");
                                    self.debug_print_all_layers().await?;
                                }
                            }
                            info!("Removed completed layer {} from list", job.layer_id);

                            if let Some(next_layer) = self.peek_current_layer().await? {
                                info!("Next layer is {}", next_layer);
                            } else {
                                info!("All layers completed (final check)!");
                            }
                        }
                        Err(e) => {
                            error!("Failed to pop layer {}: {}. Layer will remain in list.", job.layer_id, e);
                            info!("❌ Failed to pop layer, debug_print start ");
                            self.debug_print_all_layers().await?;
                            info!("❌ Failed to pop layer, debug_print end ");
                        }
                    }
                }
            }
        } else {
            debug!("Layer {} has {} remaining jobs", job.layer_id, remaining);
        }

        trace!("Job completion acknowledged successfully");
        Ok(job_status)
    }

    async fn get_current_layer_info(&self) -> Result<Option<LayerId>> {
        self.peek_current_layer().await
    }

    async fn count_pending_jobs_in_current_layer(&self) -> Result<u64> {
        let layer = self.peek_current_layer().await?.ok_or_else(|| anyhow!("No current layer"))?;

        let queue_id = self.layer_queue_id(&layer);
        self.rsmq.get_queue_length(&queue_id).await.context("Failed to get queue length")
    }

    async fn save_job_dependency_graph(&self, checkpoint_id: u64) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let serialized = bincode::serialize(&*self.job_graph.lock().await).map_err(|e| anyhow::anyhow!("Failed to serialize job graph: {}", e))?;
        let graph_key = self.graph_key(checkpoint_id);

        // Store the job graph
        conn.set(graph_key, serialized).await?;

        debug!("Job graph saved for checkpoint_id: {}", checkpoint_id);
        Ok(())
    }

    async fn clear_job_dependency_graph(&self, checkpoint_id: u64) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        conn.del(self.graph_key(checkpoint_id)).await?;
        Ok(())
    }

    async fn load_job_dependency_graph(&self, checkpoint_id: u64) -> Result<QProvingJobGraph> {
        let mut conn = self.redis_pool.get().await?;

        let graph_key = self.graph_key(checkpoint_id);

        // Check if the graph exists before trying to load it
        let exists: bool = conn.exists(&graph_key).await?;
        if !exists {
            return Err(anyhow::anyhow!(
                "Job dependency graph for checkpoint_id {} has been removed from cache",
                checkpoint_id
            ));
        }

        let graph_bytes: Vec<u8> = conn.get(graph_key).await?;
        bincode::deserialize::<QProvingJobGraph>(&graph_bytes).context("Failed to deserialize job graph")
    }

    async fn set_job_dependency_graph(
        &self,
        deploy_contracts_graph: BidirectionalGraph<QProvingJobDataID>,
        user_registrations_graph: BidirectionalGraph<QProvingJobDataID>,
        guta_graph: BidirectionalGraph<QProvingJobDataID>,
    ) -> Result<()> {
        self.job_graph.lock().await.deploy_contracts_graph = deploy_contracts_graph;
        self.job_graph.lock().await.user_registrations_graph = user_registrations_graph;
        self.job_graph.lock().await.guta_graph = guta_graph;
        Ok(())
    }

    async fn get_graph_for_job(&self, job_id: &QProvingJobDataID) -> Result<BidirectionalGraph<QProvingJobDataID>> {
        use qed_core::job::id::ProvingJobCircuitType;

        match job_id.circuit_type {
            ProvingJobCircuitType::BatchDeployContracts
            | ProvingJobCircuitType::BatchDeployContractsAggregate
            | ProvingJobCircuitType::DummyBatchDeployContractsAggregate => Ok(self.job_graph.lock().await.deploy_contracts_graph.clone()),

            ProvingJobCircuitType::AppendUserRegistrationTree
            | ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
            | ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => Ok(self.job_graph.lock().await.user_registrations_graph.clone()),

            ProvingJobCircuitType::GUTAOnlyRegisterUsers
            | ProvingJobCircuitType::GUTARegisterUsers
            | ProvingJobCircuitType::GUTATwoEndCap
            | ProvingJobCircuitType::GUTATwoGUTA
            | ProvingJobCircuitType::GUTALeftEndCapRightGUTA
            | ProvingJobCircuitType::GUTALeftGUTARightEndCap
            | ProvingJobCircuitType::GUTASingleEndCap
            | ProvingJobCircuitType::GUTAVerifyToCap
            | ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade
            | ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade
            | ProvingJobCircuitType::GUTANoChange => Ok(self.job_graph.lock().await.guta_graph.clone()),

            _ => Err(anyhow!("Job ID {:?} does not belong to any known graph", job_id)),
        }
    }

    async fn write_next_tasks(&self, task: &QProvingTask, next_task: &QProvingTask) -> Result<()> {
        let mut task_graph = self.task_graph.lock().await;
        task_graph.add_dep(next_task.clone(), task.clone());
        Ok(())
    }

    async fn write_multidimensional_tasks(&self, tasks: &[QProvingTask], next_task: &QProvingTask) -> Result<()> {
        let mut task_graph = self.task_graph.lock().await;
        let job_levels_count = tasks.len();
        for i in 0..job_levels_count {
            let current_next_task = if i == job_levels_count - 1 { next_task } else { &tasks[i + 1] };
            let current_task = &tasks[i];
            task_graph.add_dep(current_next_task.clone(), current_task.clone());
        }
        Ok(())
    }

    async fn add_task(&self, task: &QProvingTask) -> Result<()> {
        let mut task_graph = self.task_graph.lock().await;
        task_graph.add_task(task.clone());
        Ok(())
    }

    async fn get_task_graph(&self) -> QProvingTaskGraph {
        let task_graph = self.task_graph.lock().await;
        task_graph.clone()
    }

    async fn clear_task_graph(&self) -> Result<()> {
        let mut task_graph = self.task_graph.lock().await;
        task_graph.clear();
        Ok(())
    }

    async fn finalize_and_save_topology(&self) -> Result<()> {
        let task_graph = self.task_graph.lock().await;
        let layers = task_graph.ts_layers();
        tracing::debug!("Task graph layers: {:#?}", layers);
        drop(task_graph); // Release the lock before calling save_task_topology_with_layers
        self.save_task_topology_with_layers(layers).await
    }
}

// Implementation-specific methods
impl QProvingTaskStoreImpl {
    pub async fn get_job_graph_mut(&self) -> Arc<Mutex<QProvingJobGraph>> {
        self.job_graph.clone()
    }

    async fn layer_exists(&self, layer_id: &LayerId) -> Result<bool> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();
        let serialized = bincode::serialize(layer_id).context("layer_exists: Failed to serialize layer id")?;

        // ZSCORE returns the score if member exists, None otherwise
        let score: Option<f64> = conn.zscore(&layers_key, &serialized).await?;
        Ok(score.is_some())
    }
    /// Get the position (rank) of a layer in the sorted set
    async fn get_layer_rank(&self, layer_id: &LayerId) -> Result<Option<isize>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();
        let serialized = bincode::serialize(layer_id).context("get_layer_rank: Failed to serialize layer id")?;

        // ZRANK returns 0-based rank (position) in ascending order
        let rank: Option<isize> = conn.zrank(&layers_key, &serialized).await?;
        Ok(rank)
    }

    /// Check how many layers are remaining
    pub async fn get_remaining_layers_count(&self) -> Result<usize> {
        self.get_layer_count().await
    }
}

impl QProvingTaskStoreImpl {
    /// Print detailed debug information about all layers
    pub async fn debug_print_all_layers(&self) -> Result<()> {
        info!("=== Layer System Debug Report ===");

        // Get all layers
        let all_layers = self.get_all_layers().await?;
        trace!("Total layers in list: {}", all_layers.len());

        // Get current layer
        let current_layer = self.peek_current_layer().await?;
        if let Some(ref current) = current_layer {
            info!("Current layer (head): {}", current);
        } else {
            info!("No current layer (list is empty)");
        }

        // Print each layer with details
        for (index, layer) in all_layers.iter().enumerate() {
            let queue_id = self.layer_queue_id(&layer);
            let queue_stats = self.rsmq.get_queue_stats(&queue_id).await?;

            info!(
                "Layer[{}]: {} {}",
                index,
                layer,
                if current_layer.as_ref().map_or(false, |c| c == layer) {
                    ">>> CURRENT <<<"
                } else {
                    ""
                }
            );
            info!("  ID: {}", layer);
            info!("  Queue statistics:");
            info!("    - Total messages: {}", queue_stats.total_messages);
            info!("    - Visible messages: {}", queue_stats.total_messages - queue_stats.hidden_messages);
            info!("    - Hidden messages: {}", queue_stats.hidden_messages);
            info!("    - Total sent: {}", queue_stats.total_sent);
            info!("    - Total received: {}", queue_stats.total_received);
        }

        info!("=== End Debug Report ===");
        Ok(())
    }
}


/// Get current timestamp in milliseconds
pub fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
