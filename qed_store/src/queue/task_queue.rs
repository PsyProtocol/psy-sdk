use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use scylla::_macro_internal::SerializeRow;
use tracing::{debug, error, info, trace, warn};
use qed_core::job::id::{JobsLayer, JobsTask, JobsTaskGraph, QProvingJobDataID, TaskId};
use crate::queue::{new_redis_async_pool, QueueId, QueueStats, RsmqQueue};

const TASK_COMMON_PREFIX: &str = "tasks:";
const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(30);


pub type LayerId = TaskId;

/// Represents a layer of tasks that can be executed in parallel
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskLayer {
    pub layer_id: LayerId,
    pub task_ids: Vec<TaskId>,
}

impl TaskLayer {
    pub fn new(layer_id: LayerId, task_ids: Vec<TaskId>) -> Self {
        Self { layer_id, task_ids }
    }

    pub fn is_empty(&self) -> bool {
        self.task_ids.is_empty()
    }

    pub fn contains(&self, task_id: &TaskId) -> bool {
        self.task_ids.contains(task_id)
    }
}

/// Represents a single proving job with task assignment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QJob {
    pub job_id: QProvingJobDataID,
    pub layer_id: LayerId,
    #[serde(default)]
    pub msg_id: String,
}

impl QJob {
    /// Create a new QJob
    pub fn new(job_id: QProvingJobDataID, layer_id: LayerId) -> Self {
        Self {
            job_id,
            layer_id,  // Default to a new TaskId
            msg_id: String::new(),
        }
    }

    /// Set the message ID (builder pattern)
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

/// Job task store implementation with Redis backend and layer support
pub struct JobTaskStoreImpl {
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    rsmq: Arc<RsmqQueue>,
    checkpoint_id: u64,  // Store the checkpoint ID
}

impl JobTaskStoreImpl {
    pub async fn new(redis_url: &str, pool_size: usize, checkpoint_id: u64) -> Result<Self> {
        debug!("Initializing JobTaskStore for checkpoint {} with pool size {}", checkpoint_id, pool_size);

        let redis_pool = Arc::new(
            new_redis_async_pool(redis_url, pool_size)
                .await
                .context("Failed to create Redis pool")?
        );

        // Use the unified RsmqQueue instead of RsmqTaskQueue
        let rsmq = Arc::new(
            RsmqQueue::new(redis_url, pool_size, format!("job_task_store:{}", checkpoint_id))
                .await
                .context("Failed to create RSMQ queue")?
        );

        Ok(Self {
            redis_pool,
            rsmq,
            checkpoint_id,
        })
    }

    /// Get the checkpoint-specific graph key
    #[inline]
    fn graph_key(&self, checkpoint_id: u64) -> String {
        format!("{}:{}:graph", TASK_COMMON_PREFIX, checkpoint_id)
    }

    /// Get the checkpoint-specific layers key
    #[inline]
    fn layers_key(&self) -> String {
        format!("{}:{}:layer_lists", TASK_COMMON_PREFIX, self.checkpoint_id)
    }

    /// Generate queue name for a layer (includes checkpoint)
    #[inline]
    fn layer_queue_name(&self, layer_id: &TaskId) -> String {
        format!("{}:{}:{}:rsmq", TASK_COMMON_PREFIX, self.checkpoint_id, layer_id)
    }

    /// Create QueueId for a layer
    #[inline]
    fn layer_queue_id(&self, layer_id: &TaskId) -> QueueId {
        QueueId::WorkerEvent {
            queue_biz_key: self.layer_queue_name(layer_id),
        }
    }

    /// Push layers to the tail of the list
    async fn push_layers(&self, layers: &[JobsLayer]) -> Result<()> {
        if layers.is_empty() {
            return Ok(());
        }

        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        // Push each layer to the tail of the list
        for layer in layers {
            let task_layer = TaskLayer::new(layer.layer_id, layer.task_ids.clone());
            let serialized = bincode::serialize(&task_layer)?;
            conn.rpush(&layers_key, serialized).await?;
        }

        debug!("Pushed {} layers to list for checkpoint {}", layers.len(), self.checkpoint_id);
        Ok(())
    }

    /// Peek at the current layer (head of list) without removing
    async fn peek_current_layer(&self) -> Result<Option<TaskLayer>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        // Get the first element without removing it
        let bytes: Option<Vec<u8>> = conn.lindex(&layers_key, 0).await?;

        match bytes {
            Some(data) => {
                let layer = bincode::deserialize(&data)?;
                Ok(Some(layer))
            }
            None => Ok(None),
        }
    }

    /// Pop the current layer only if it matches the expected layer ID
    async fn pop_current_layer(&self, expected_layer_id: &TaskId) -> Result<Option<TaskLayer>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        // First, peek at the current layer to check if it matches
        let peek_bytes: Option<Vec<u8>> = conn.lindex(&layers_key, 0).await?;

        match peek_bytes {
            Some(data) => {
                let layer: TaskLayer = bincode::deserialize(&data)?;

                // Check if the layer ID matches the expected one
                if layer.layer_id != *expected_layer_id {
                    warn!(
                        "Layer mismatch for checkpoint {}: expected {}, found {}. Abandoning pop operation.",
                        self.checkpoint_id, expected_layer_id, layer.layer_id
                    );
                    return Err(anyhow!(
                        "Layer ID mismatch: expected {}, found {}",
                        expected_layer_id,
                        layer.layer_id
                    ));
                }

                // Layer matches, now actually pop it
                let pop_bytes: Option<Vec<u8>> = conn.lpop(&layers_key, None).await?;

                match pop_bytes {
                    Some(pop_data) => {
                        let popped_layer: TaskLayer = bincode::deserialize(&pop_data)?;
                        info!(
                            "Successfully popped layer {} from checkpoint {}",
                            popped_layer.layer_id,
                            self.checkpoint_id
                        );
                        Ok(Some(popped_layer))
                    }
                    None => {
                        // This shouldn't happen, but handle it gracefully
                        error!("Layer disappeared between peek and pop for checkpoint {}", self.checkpoint_id);
                        Ok(None)
                    }
                }
            }
            None => {
                debug!("No layers to pop for checkpoint {}", self.checkpoint_id);
                Ok(None)
            }
        }
    }

    /// Force pop the current layer without checking (for emergency/admin use)
    async fn force_pop_current_layer(&self) -> Result<Option<TaskLayer>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();

        let bytes: Option<Vec<u8>> = conn.lpop(&layers_key, None).await?;

        match bytes {
            Some(data) => {
                let layer: TaskLayer = bincode::deserialize(&data)?;
                warn!("Force popped layer {} from checkpoint {}", layer.layer_id, self.checkpoint_id);
                Ok(Some(layer))
            }
            None => Ok(None),
        }
    }

    /// Get the total number of remaining layers
    async fn get_layer_count(&self) -> Result<usize> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();
        let count: usize = conn.llen(&layers_key).await?;
        Ok(count)
    }

    /// Get all layers without removing them (for monitoring)
    async fn get_all_layers(&self) -> Result<Vec<TaskLayer>> {
        let mut conn = self.redis_pool.get().await?;
        let layers_key = self.layers_key();
        let all_bytes: Vec<Vec<u8>> = conn.lrange(&layers_key, 0, -1).await?;

        let mut layers = Vec::with_capacity(all_bytes.len());
        for bytes in all_bytes {
            layers.push(bincode::deserialize(&bytes)?);
        }

        Ok(layers)
    }

    /// Check if a layer is complete
    async fn is_layer_complete(&self, layer_id: &TaskId) -> Result<bool> {
        let queue_id = self.layer_queue_id(layer_id);
        let count = self.rsmq.get_queue_length(&queue_id).await?;
        Ok(count == 0)
    }

    /// Get queue statistics for monitoring
    pub async fn get_queue_stats(&self) -> Result<HashMap<TaskId, u64>> {
        let layers = self.get_all_layers().await?;
        let mut stats = HashMap::with_capacity(layers.len());

        for layer in layers {
            let queue_id = self.layer_queue_id(&layer.layer_id);
            if let Ok(count) = self.rsmq.get_queue_length(&queue_id).await {
                stats.insert(layer.layer_id, count);  // Now correctly using TaskId as key
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
            status.push_str(&format!("Current Layer: {}\n", current.layer_id));
        } else {
            status.push_str("Current Layer: None (all completed)\n");
        }

        for layer in layers {
            let count = stats.get(&layer.layer_id).copied().unwrap_or(0);
            status.push_str(&format!(
                "  Layer {}: {} tasks, {} pending jobs\n",
                layer.layer_id,
                layer.task_ids.len(),
                count
            ));
        }

        Ok(status)
    }

    /// Clear all tasks and queues
    pub async fn clear_all(&self) -> Result<()> {
        let layers = self.get_all_layers().await?;

        // Delete all layer queues
        for layer in layers {
            let queue_id = self.layer_queue_id(&layer.layer_id);
            let _ = self.rsmq.delete_queue(&queue_id).await;
        }

        let mut conn = self.redis_pool.get().await?;
        conn.del(&self.layers_key()).await?;
        conn.del(&self.graph_key(self.checkpoint_id)).await?;

        info!("Cleared all layers and queues for checkpoint {}", self.checkpoint_id);
        Ok(())
    }

    /// Convert topologically sorted layers to TaskLayers
    fn create_task_layers(layers: Vec<Vec<TaskId>>) -> Vec<TaskLayer> {
        layers.into_iter()
            .map(|task_ids| {
                // Generate a unique TaskId for this layer
                let layer_id = TaskId::new();
                TaskLayer::new(layer_id, task_ids)
            })
            .collect()
    }
}

#[async_trait]
pub trait JobTaskStore {
    async fn save_task_topology_with_layers(&self, graph: Vec<JobsLayer>) -> Result<()>;
    async fn claim_job_from_current_layer(&self) -> Result<Option<QJob>>;
    async fn acknowledge_job_completion(&self, job: &QJob) -> Result<()>;
    async fn get_current_layer_info(&self) -> Result<Option<TaskLayer>>;
    async fn count_pending_jobs_in_current_layer(&self) -> Result<u64>;

    // Legacy operations (kept for compatibility)
    async fn save_job_dependency_graph(&self, graph: &JobsTaskGraph) -> Result<()>;
    async fn load_job_dependency_graph(&self, checkpoint_id: u64) -> Result<JobsTaskGraph> ;
}

#[async_trait]
impl JobTaskStore for JobTaskStoreImpl {
    async fn save_task_topology_with_layers(&self, layers: Vec<JobsLayer>) -> Result<()> {
        info!("Saving task topology with layer support");

        // Step 1: Create an RSMQ queue for each layer and send jobs
        for layer in &layers {
            // Create queue for this layer
            let queue_id = self.layer_queue_id(&layer.layer_id);
            self.rsmq.create_queue_if_not_exists(&queue_id).await?;

            // Create QJob instances for all job IDs in this layer
            let jobs: Vec<QJob> = layer.job_ids
                .iter()
                .map(|job_id| QJob::new(job_id.clone(), layer.layer_id))
                .collect();

            // Send all jobs to the corresponding layer queue
            if !jobs.is_empty() {
                self.rsmq.send_batch(&queue_id, &jobs).await?;
                info!("Sent {} jobs to layer {} queue", jobs.len(), layer.layer_id);
            }
        }

        // Step 2: Clear existing layers list for this checkpoint
        let mut conn = self.redis_pool.get().await?;
        conn.del(&self.layers_key()).await?;

        // Step 3: Send all layers to the Redis list
        self.push_layers(&layers).await?;

        info!("Successfully saved {} layers", layers.len());
        Ok(())
    }

    async fn claim_job_from_current_layer(&self) -> Result<Option<QJob>> {
        // Peek at the current layer (head of the list)
        let current_layer = match self.peek_current_layer().await? {
            Some(layer) => layer,
            None => {
                debug!("No layers available");
                return Ok(None);
            }
        };

        let queue_id = self.layer_queue_id(&current_layer.layer_id);

        // Try to claim a job from the current layer's merged queue
        match self.rsmq.receive_object_with_id::<QJob>(&queue_id, Some(VISIBILITY_TIMEOUT)).await? {
            Some((job, msg_id)) => {
                debug!("Claimed {} from layer {}", job, current_layer.layer_id);
                Ok(Some(job.with_msg_id(msg_id)))
            }
            None => {
                // The Current layer queue is empty, check if layer is complete
                if self.is_layer_complete(&current_layer.layer_id).await? {
                    info!("Layer {} completed, removing from list", current_layer.layer_id);

                    // Pop the completed layer from head with verification
                    match self.pop_current_layer(&current_layer.layer_id).await {
                        Ok(_) => {
                            // Successfully popped, try to claim from next layer
                            if self.peek_current_layer().await?.is_some() {
                                Box::pin(self.claim_job_from_current_layer()).await
                            } else {
                                info!("All layers completed");
                                Ok(None)
                            }
                        }
                        Err(e) => {
                            error!("Failed to pop layer: {}. Abandoning task.", e);
                            Ok(None)
                        }
                    }
                } else {
                    // Layer not complete but no jobs available (might be processing)
                    Ok(None)
                }
            }
        }
    }

    async fn acknowledge_job_completion(&self, job: &QJob) -> Result<()> {
        debug!("Acknowledging {}", job);

        let queue_id = self.layer_queue_id(&job.layer_id);

        // Delete message from queue,
        // note: it should after the proof has been verified
        self.rsmq.delete_message(&queue_id, &job.msg_id).await?;

        // Check if the layer is complete
        let remaining = self.rsmq.get_queue_length(&queue_id).await?;

        if remaining == 0 {
            info!("Layer {} has no more jobs", job.layer_id);

            // Check if this is the current layer and remove it if complete
            if let Some(current_layer) = self.peek_current_layer().await? {
                if current_layer.layer_id == job.layer_id && self.is_layer_complete(&job.layer_id).await? {
                    match self.pop_current_layer(&job.layer_id).await {
                        Ok(_) => {
                            info!("Removed completed layer {} from list", job.layer_id);

                            if let Some(next_layer) = self.peek_current_layer().await? {
                                info!("Next layer is {}", next_layer.layer_id);
                            } else {
                                info!("All layers completed!");
                            }
                        }
                        Err(e) => {
                            error!("Failed to pop layer {}: {}. Layer will remain in list.", job.layer_id, e);
                        }
                    }
                }
            }
        } else {
            debug!("Layer {} has {} remaining jobs", job.layer_id, remaining);
        }

        info!("Topology saved successfully");
        Ok(())
    }

    async fn get_current_layer_info(&self) -> Result<Option<TaskLayer>> {
        self.peek_current_layer().await
    }

    async fn count_pending_jobs_in_current_layer(&self) -> Result<u64> {
        let layer = self.peek_current_layer().await?
            .ok_or_else(|| anyhow!("No current layer"))?;

        let queue_id = self.layer_queue_id(&layer.layer_id);
        self.rsmq.get_queue_length(&queue_id).await
            .context("Failed to get queue length")
    }

    async fn save_job_dependency_graph(&self, graph: &JobsTaskGraph) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let serialized = bincode::serialize(graph)?;
        let graph_key = self.graph_key(self.checkpoint_id);
        conn.set(graph_key, serialized).await?;

        debug!("Job graph saved");
        Ok(())
    }

    async fn load_job_dependency_graph(&self, checkpoint_id: u64) -> Result<JobsTaskGraph> {
        let mut conn = self.redis_pool.get().await?;

        let graph_key = self.graph_key(checkpoint_id);
        let graph_bytes: Vec<u8> = conn.get(graph_key).await?;

        bincode::deserialize::<JobsTaskGraph>(&graph_bytes)
            .context("Failed to deserialize job graph")
    }
}

// Implementation-specific methods
impl JobTaskStoreImpl {
    /// Compute layers from graph (placeholder - your colleague will provide the actual implementation)
    async fn compute_layers_from_graph(&self, graph: &JobsTaskGraph) -> Result<Vec<Vec<TaskId>>> {
        // This is a placeholder implementation
        // Your colleague will modify the ts() function to return Vec<Vec<TaskId>>
        // For now, simulating a simple layer computation

        let mut layers = Vec::new();
        let mut visited = HashSet::new();
        let mut current_layer = Vec::new();

        // Find all tasks with no dependencies (first layer)
        for task_id in graph.tasks.keys() {
            if !graph.deps_on.contains_key(task_id) {
                current_layer.push(*task_id);
                visited.insert(*task_id);
            }
        }

        if !current_layer.is_empty() {
            layers.push(current_layer);
        }

        // Build subsequent layers
        while visited.len() < graph.tasks.len() {
            let mut next_layer = Vec::new();

            for task_id in graph.tasks.keys() {
                if !visited.contains(task_id) {
                    // Check if all dependencies are satisfied
                    if let Some(deps) = graph.deps.get(task_id) {
                        if deps.iter().all(|dep| visited.contains(dep)) {
                            next_layer.push(*task_id);
                        }
                    }
                }
            }

            if next_layer.is_empty() {
                break; // No more tasks can be scheduled
            }

            for task_id in &next_layer {
                visited.insert(*task_id);
            }

            layers.push(next_layer);
        }

        Ok(layers)
    }

    /// Get status of a specific layer
    pub async fn get_layer_status(&self, layer_id: LayerId) -> Result<(usize, u64)> {
        let layers = self.get_all_layers().await?;
        let layer = layers.iter()
            .find(|l| l.layer_id == layer_id)
            .ok_or_else(|| anyhow!("Layer {} not found", layer_id))?;

        let queue_id = self.layer_queue_id(&layer_id);
        let pending = self.rsmq.get_queue_length(&queue_id).await?;

        Ok((layer.task_ids.len(), pending))
    }

    /// Manually remove current layer (for admin/debugging)
    pub async fn force_complete_current_layer(&self) -> Result<()> {
        if let Some(layer) = self.force_pop_current_layer().await? {
            info!("Forcefully removed layer {} from list", layer.layer_id);

            // Also clear its queue
            let queue_id = self.layer_queue_id(&layer.layer_id);
            let _ = self.rsmq.delete_queue(&queue_id).await;
        }
        Ok(())
    }

    /// Get all jobs from current layer (for monitoring)
    pub async fn peek_current_layer_jobs(&self) -> Result<Vec<QJob>> {
        let layer = self.peek_current_layer().await?
            .ok_or_else(|| anyhow!("No current layer"))?;

        let queue_id = self.layer_queue_id(&layer.layer_id);
        // Note: This will drain the queue, use carefully!
        self.rsmq.pop_all::<QJob>(&queue_id).await
    }

    /// Check how many layers are remaining
    pub async fn get_remaining_layers_count(&self) -> Result<usize> {
        self.get_layer_count().await
    }

    /// Get the checkpoint ID this store is managing
    pub fn checkpoint_id(&self) -> u64 {
        self.checkpoint_id
    }
}

