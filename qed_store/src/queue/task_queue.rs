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

const TASK_COMMON_PREFIX: &str = "tasks";
pub const JOB_STATUS_PREFIX: &str = "js"; // short for job-status
pub const JOB_TIMEOUT_PREFIX: &str = "jt";

pub const JOB_STATUS_TTL_SECONDS: u64 = 7200; //job status ttl in seconds, 2 hour
pub const JOB_EXECUTION_TIMEOUT_SECONDS: u64 =  30;

const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(30);


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
    biz_key: String,
    task_graph: Arc<Mutex<QProvingTaskGraph>>,
    job_graph: Arc<Mutex<QProvingJobGraph>>,
}

impl QProvingTaskStoreImpl {
    pub async fn new(redis_url: &str, pool_size: usize, queue_biz_key: &str) -> Result<Self> {
        debug!("Initializing QProvingTaskStore with pool size {}", pool_size);
        trace!("redis_url: {}, queue_biz_key: {}", redis_url, queue_biz_key);

        let redis_pool = Arc::new(new_redis_async_pool(redis_url, pool_size)
            .await.context("Failed to create Redis pool")?);

        let rsmq = Arc::new(
            RsmqQueue::new(redis_url, pool_size, queue_biz_key)
                .await
                .context("Failed to create RSMQ queue")?,
        );

        Ok(Self {
            redis_pool,
            rsmq,
            biz_key: queue_biz_key.to_string(),
            task_graph: Arc::new(Mutex::new(QProvingTaskGraph::new())),
            job_graph: Arc::new(Mutex::new(QProvingJobGraph::new())),
        })
    }

    #[inline]
    fn graph_key(&self, checkpoint_id: u64) -> String {
        format!("{}:{}:{}:graph", &self.biz_key, TASK_COMMON_PREFIX, checkpoint_id)
    }

    #[inline]
    fn layer_zset_key(&self) -> String {
        format!("{}:{}:layer_zset",  &self.biz_key, TASK_COMMON_PREFIX)
    }

    #[inline]
    fn layer_task_queue_key(&self, layer_id: &LayerId) -> String {
        format!("{}:{}:{}:rsmq", &self.biz_key, TASK_COMMON_PREFIX, layer_id)
    }

    #[inline]
    pub fn job_status_key(&self, job_id: &QProvingJobDataID) -> String {
        format!("{}:{}:{}", &self.biz_key, JOB_STATUS_PREFIX, job_id.to_hex_string())
    }

    #[inline]
    pub fn job_timeout_key(&self,job_id: &QProvingJobDataID) -> String {
        format!("{}:{}:{}", &self.biz_key, JOB_TIMEOUT_PREFIX, job_id.to_hex_string())
    }

    #[inline]
    pub fn job_timeout_to_status(timeout_key: &str) -> Option<String> {
        if let Some(pos) = timeout_key.find(&format!(":{}", JOB_TIMEOUT_PREFIX)) {
            let biz_prefix = &timeout_key[..pos];
            trace!("Job timeout biz prefix: {}", biz_prefix);
            if let Some(rest) = timeout_key.strip_prefix(&format!("{}:", biz_prefix)) {
                if let Some(rest2) = rest.strip_prefix(&format!("{}:", JOB_TIMEOUT_PREFIX)) {
                    //todo! debug only, should use return directly
                    let ret = format!("{}:{}:{}", biz_prefix, JOB_STATUS_PREFIX, rest2);
                    trace!("Job status key : {}", ret);
                    return Some(ret);
                    // return Some(format!("{}:{}:{}", biz_prefix, JOB_STATUS_PREFIX, rest2));
                }
            }
        }
        None
    }

    #[inline]
    pub fn job_status_to_timeout(status_key: &str) -> Option<String> {
        if let Some(pos) = status_key.find(&format!(":{}", JOB_STATUS_PREFIX)) {
            let biz_prefix = &status_key[..pos];
            trace!("Job status biz prefix: {}", biz_prefix);
            if let Some(rest) = status_key.strip_prefix(&format!("{}:", biz_prefix)) {
                if let Some(rest2) = rest.strip_prefix(&format!("{}:", JOB_STATUS_PREFIX)) {
                    //todo! debug only, should use return directly
                    let ret = format!("
                        {}:{}:{}", biz_prefix, JOB_TIMEOUT_PREFIX, rest2);
                    trace!("Job timeout key : {}", ret);
                    return Some(ret);

                    // return Some(format!("{}:{}:{}", biz_prefix, JOB_TIMEOUT_PREFIX, rest2));
                }
            }
        }
        None
    }

    async fn set_job_status(&self, status: &QJobStatus) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key =  self.job_status_key(&status.id);
        let value = status.to_bytes()?;
        debug!("Setting job status in Redis: key={}, status={:?}", key, status);
        conn.set_ex(key, value, JOB_STATUS_TTL_SECONDS).await?;
        Ok(())
    }

    pub async fn get_job_status(&self, job_id: &QProvingJobDataID) -> Result<Option<QJobStatus>> {
        let mut conn = self.redis_pool.get().await?;
        let key =  self.job_status_key(job_id);
        let value: Option<Vec<u8>> = conn.get(key).await?;
        match value {
            Some(bytes) => Ok(Some(QJobStatus::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    async fn set_job_timeout(&self, job_id: &QProvingJobDataID, timeout_seconds: u64) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key =  self.job_timeout_key(job_id);
        // Set empty value with expiration
        conn.set_ex(key, "", timeout_seconds).await?;
        Ok(())
    }

    async fn delete_job_timeout(&self, job_id: &QProvingJobDataID) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key = self.job_timeout_key(job_id);
        conn.del(key).await?;
        Ok(())
    }

    #[inline]
    fn layer_queue_id(&self, layer_id: &LayerId) -> QueueId {
        QueueId::WorkerEvent {
            queue_biz_key: self.layer_task_queue_key(layer_id),
        }
    }

    /// Push layers to the tail of the list
    async fn push_layers(&self, layers: &[QProvingTaskLayer]) -> Result<()> {
        if layers.is_empty() {
            return Ok(());
        }

        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layer_zset_key();

        /* Note: If different nodes push at the same time, there may be problems due to clock asynchrony.
           In our current scenario, only the processor will push, so there is no problem. */
        // Use timestamp + sequence for unique, ordered scores
        let base_score = chrono::Utc::now().timestamp_micros();

        let mut pipe = redis::pipe();
        pipe.atomic();

        for (idx, layer) in layers.iter().enumerate() {
            let bytes = layer.layer_id.to_bytes();
            let score = base_score + (idx as i64);
            pipe.zadd(&layers_key, &bytes, score);
            trace!("Pipe key={}, score={}", layers_key, score);
        }

        pipe.query_async(&mut *conn).await?;

        debug!("Pushed {} layers to sorted set", layers.len());
        Ok(())
    }

    async fn peek_current_layer(&self) -> Result<Option<LayerId>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layer_zset_key();
        trace!("Peeking current layer from key={}", layers_key);
        // Get the first element without removing it
        let result: Vec<Vec<u8>> = conn.zrange(&layers_key, 0, 0).await?;

        if let Some(data) = result.first() {
            match LayerId::from_slice(data) {
                Ok(layer_id) => Ok(Some(layer_id)),
                Err(e) => {
                    error!("Failed to parse LayerId from Redis data: {:?}, data = {}", e, hex::encode(data));
                    Err(e)
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn pop_current_layer(&self, target_layer_id: &LayerId) -> Result<bool> {
        let mut conn = self.redis_pool.get().await?;
        let layer_zset_key = self.layer_zset_key();

        match self.get_layer_rank(target_layer_id).await? {
            Some(0) => {
                let removed: u64 = conn.zrem(&layer_zset_key, &target_layer_id.to_bytes()).await?;
                if removed > 0 {
                    trace!("✅ Popped layer {:?} from head", target_layer_id);
                    Ok(true)
                } else {
                    debug!(
                        "ℹ️ Concurrent pop detected: layer {:?} was already removed by another edge",
                        target_layer_id
                    );
                    Ok(false)
                }
            }
            Some(rank) => {
                debug!(
                    "⏭️ Skipped pop: layer {:?} is at position {}, not at head",
                    target_layer_id, rank
                );
                Ok(false)
            }
            None => {
                debug!(
                    "🗑️ Layer {:?} not found in current queue (already processed or expired)",
                    target_layer_id
                );
                Ok(false)
            }
        }
    }

    async fn get_layer_count(&self) -> Result<usize> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layer_zset_key();
        let count: usize = conn.zcard(&layers_key).await?;
        Ok(count)
    }

    async fn get_all_layers(&self) -> Result<Vec<LayerId>> {
        let mut conn = self.redis_pool.get().await?;
        let key = self.layer_zset_key();

        let all_bytes: Vec<Vec<u8>> = conn.zrange(&key, 0, -1).await?;

        let layers = all_bytes
            .into_iter()
            .filter_map(|bytes| match LayerId::from_slice(&bytes) {
                Ok(id) => Some(id),
                Err(e) => {
                    warn!("Failed to parse LayerId from Redis bytes: {:?}", e);
                    None
                }
            })
            .collect();

        Ok(layers)
    }

    async fn is_layer_complete(&self, layer_id: &LayerId) -> Result<bool> {
        let queue_id = self.layer_queue_id(layer_id);
        let stats = self.rsmq.get_queue_stats(&queue_id).await?;

        if stats.total_messages == 0 && stats.hidden_messages == 0 {
            return Ok(true);
        }

        if stats.total_messages == 0 && stats.hidden_messages > 0 {
            info!(
                "⏳ Layer {} is still being processed ({} hidden messages in-flight)",
                layer_id, stats.hidden_messages
            );
        } else if stats.total_messages > 0 {
            debug!(
                "📥 Layer {} still has {} pending messages in queue",
                layer_id, stats.total_messages
            );
        }

        Ok(false)
    }
}

impl QProvingTaskStoreImpl {
    /// Validates job ownership and extends its visibility timeout if valid
    pub async fn validate_and_extend_job(&self, job: &QJob) -> Result<JobValidationStatus> {

        let current_layer = match self.peek_current_layer().await? {
            Some(layer) => layer,
            None => {
                warn!("No active layer when validating job {}", job);
                return Ok(JobValidationStatus::NoActiveLayer);
            }
        };

        if current_layer != job.layer_id {
            warn!("Job {} claimed in layer {} but current layer is {}", job, job.layer_id, current_layer);
            return Ok(JobValidationStatus::WrongLayer {
                expected: current_layer,
                provided: job.layer_id,
            });
        }

        let queue_id = self.layer_queue_id(&job.layer_id);
        match self.rsmq.change_message_visibility(&queue_id, &job.msg_id, VISIBILITY_TIMEOUT).await {
            Ok(_) => {
                debug!("Job {} validated: message {} is hidden and visibility extended", job, job.msg_id);
                Ok(JobValidationStatus::Valid)
            }
            Err(e) => classify_visibility_error(job, e),
        }
    }

    /// Simplified validation that just returns true/false
    pub async fn is_job_valid(&self, job: &QJob) -> Result<bool> {
        match self.validate_and_extend_job(job).await? {
            JobValidationStatus::Valid => Ok(true),
            _ => Ok(false),
        }
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

fn classify_visibility_error(job: &QJob, error: anyhow::Error) -> Result<JobValidationStatus> {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("not found") || error_str.contains("does not exist") {
        warn!("Job {} validation failed: message {} not found in queue", job, job.msg_id);
        return Ok(JobValidationStatus::MessageNotFound);
    }

    if error_str.contains("visible") || error_str.contains("not hidden") {
        warn!("Job {} validation failed: message {} is visible (not being processed)", job, job.msg_id);
        return Ok(JobValidationStatus::MessageNotHidden);
    }

    error!("Unexpected error validating job {}: {}", job, error);
    Err(error)
}

#[derive(Debug, PartialEq, Clone)]
pub enum JobValidationStatus {
    Valid,
    NoActiveLayer,
    WrongLayer { expected: LayerId, provided: LayerId },
    MessageNotFound,
    MessageNotHidden,
}


#[async_trait]
pub trait QProvingTaskStore {
    async fn initialize_task_topology(&self, graph: Vec<QProvingTaskLayer>) -> Result<()>;
    async fn acquire_job(&self, worker_id: &str) -> Result<Option<QJob>>;
    async fn mark_job_completed(&self, job: &QJob, worker_id: &str) -> Result<QJobStatus>;

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
    async fn handle_layer_completion(&self, layer_id: &LayerId) -> Result<()>;
}

#[async_trait]
impl QProvingTaskStore for QProvingTaskStoreImpl {
    async fn initialize_task_topology(&self, layers: Vec<QProvingTaskLayer>) -> Result<()> {

        if layers.is_empty() {
            debug!("No layers to save, skipping topology save");
            return Ok(());
        }
        info!("Saving {} layers with {} total jobs",
            layers.len(),
            layers.iter().map(|l| l.job_ids.len()).sum::<usize>()
        );

        for layer in &layers {
            let queue_id = self.layer_queue_id(&layer.layer_id);
            self.rsmq.create_queue_if_not_exists(&queue_id).await?;

            if !layer.job_ids.is_empty() {
                let jobs: Vec<QJob> = layer.job_ids
                    .iter()
                    .map(|job_id| QJob::new(job_id.clone(), layer.layer_id))
                    .collect();

                self.rsmq.send_batch(&queue_id, &jobs).await?;
                debug!("Layer {}: enqueued {} jobs", layer.layer_id, jobs.len());
            } else {
                warn!("Layer {} has no jobs to enqueue", layer.layer_id);
            }
        }

        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layer_zset_key();
        let deleted: u64 = conn.del(&layers_key).await?;
        if deleted > 0 {
            debug!("Cleared existing layer queue (removed {} entries)", deleted);
        }

        self.push_layers(&layers).await?;
        info!("Topology saved: {} layers ready for processing", layers.len());

        Ok(())
    }

    async fn acquire_job(&self, worker_id: &str) -> Result<Option<QJob>> {
        let current_layer = match self.peek_current_layer().await? {
            Some(layer) => layer,
            None => {
                trace!("No layers available");
                return Ok(None);
            }
        };

        let queue_id = self.layer_queue_id(&current_layer);

        let (mut job, msg_id) = match self.rsmq.receive_object_with_id::<QJob>(&queue_id, Some(VISIBILITY_TIMEOUT)).await? {
            Some(result) => result,
            None => {
                trace!("Layer {} queue is empty", current_layer);
                return Ok(None);
            }
        };

        job = job.with_msg_id(msg_id);

        let job_status = match self.get_job_status(&job.job_id).await? {
            Some(mut status) => {
                warn!("Reclaiming job {} from worker {:?} to worker {}",
                job.job_id, status.worker_id, worker_id);
                status.reset_for_reclaim(worker_id);
                status
            }
            None => QJobStatus::new_processing(job.job_id.clone(), worker_id)
        };

        self.set_job_status(&job_status).await?;
        self.set_job_timeout(&job.job_id, JOB_EXECUTION_TIMEOUT_SECONDS).await?;

        info!("Worker {} got job {} from layer {}", worker_id, job.job_id, current_layer);

        Ok(Some(job))
    }

    async fn mark_job_completed(&self, job: &QJob, worker_id: &str) -> Result<QJobStatus> {
        let queue_id = self.layer_queue_id(&job.layer_id);

        // Get current job status
        let mut job_status = self.get_job_status(&job.job_id).await?
            .ok_or_else(|| anyhow!("Job status not found for {}", job.job_id))?;

        validate_worker_authority(&job_status, job, worker_id)?;

        if job_status.status != Status::Processing {
            return Err(anyhow!(
                "Job {} is not in Processing state (current: {:?})",
                job.job_id, job_status.status
            ));
        }

        job_status.mark_completed();
        self.set_job_status(&job_status).await?;
        self.delete_job_timeout(&job.job_id).await?;
        self.rsmq.delete_message(&queue_id, &job.msg_id).await?;

        let remaining = self.rsmq.get_queue_length(&queue_id).await?;
        if remaining == 0 {
            self.handle_layer_completion(&job.layer_id).await?;
        } else {
            debug!("Layer {} has {} remaining jobs", job.layer_id, remaining);
        }
        info!("Worker {} completed job {} from layer {}", worker_id, job.job_id, job.layer_id);
        Ok(job_status)
    }

    async fn handle_layer_completion(&self, layer_id: &LayerId) -> Result<()> {
        let current_layer = match self.peek_current_layer().await? {
            Some(layer) => layer,
            None => return Ok(()),
        };

        if current_layer != *layer_id || !self.is_layer_complete(layer_id).await? {
            return Ok(());
        }

        match self.pop_current_layer(layer_id).await? {
            true => {
                info!("Completed and removed layer {}", layer_id);

                match self.peek_current_layer().await? {
                    Some(next) => info!("Advancing to layer {}", next),
                    None => info!("✅ All layers completed!"),
                }
            }
            false => {
                debug!("Layer {} already removed by another edge", layer_id);
            }
        }

        Ok(())
    }

    async fn save_job_dependency_graph(&self, checkpoint_id: u64) -> Result<()> {
        let graph = self.job_graph.lock().await;
        let serialized = bincode::serialize(&*graph)?;
        drop(graph);


        let mut conn = self.redis_pool.get().await?;
        let graph_key = self.graph_key(checkpoint_id);
        conn.set(graph_key, serialized).await?;

        debug!("Saved job graph for checkpoint {}", checkpoint_id);
        Ok(())
    }

    async fn clear_job_dependency_graph(&self, checkpoint_id: u64) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key = self.graph_key(checkpoint_id);
        let deleted: u64 = conn.del(&key).await?;
        if deleted > 0 {
            debug!("Cleared job graph for checkpoint {}", checkpoint_id);
        }
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
        let mut graph = self.job_graph.lock().await;
        graph.deploy_contracts_graph = deploy_contracts_graph;
        graph.user_registrations_graph = user_registrations_graph;
        graph.guta_graph = guta_graph;
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
        self.initialize_task_topology(layers).await
    }
}

// Implementation-specific methods
impl QProvingTaskStoreImpl {
    pub async fn get_job_graph_mut(&self) -> Arc<Mutex<QProvingJobGraph>> {
        self.job_graph.clone()
    }

    async fn layer_exists(&self, layer_id: &LayerId) -> Result<bool> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layer_zset_key();
        let serialized = bincode::serialize(layer_id)?;

        // ZSCORE returns the score if member exists, None otherwise
        let score: Option<f64> = conn.zscore(&layers_key, &serialized).await?;
        Ok(score.is_some())
    }
    async fn get_layer_rank(&self, layer_id: &LayerId) -> Result<Option<isize>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layer_zset_key();
        let bytes = layer_id.to_bytes();

        // ZRANK returns 0-based rank (position) in ascending order
        let rank: Option<isize> = conn.zrank(&layers_key, &bytes).await?;
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

fn validate_worker_authority(status: &QJobStatus, job: &QJob, worker_id: &str) -> Result<()> {
    match &status.worker_id {
        Some(claimed_id) if claimed_id == worker_id => Ok(()),
        Some(claimed_id) => Err(anyhow!(
            "Worker mismatch: job {} claimed by {} but {} attempted completion",
            job.job_id, claimed_id, worker_id
        )),
        None => Err(anyhow!(
            "Job {} has no worker ID - cannot verify completion authority",
            job.job_id
        )),
    }
}