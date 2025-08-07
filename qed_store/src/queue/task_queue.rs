use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, trace, warn};

use qed_core::job::id::{JobsTask, JobsTaskGraph, QProvingJobDataID, TaskId};
use crate::queue::{new_redis_async_pool, QueueId, QueueStats, RsmqQueue};

// Configuration constants
const JOB_GRAPH_KEY: &str = "job_graph:";
const TASK_LIST_KEY: &str = "task:list:";
const TASK_RSMQ_PREFIX: &str = "task:rsmq:";
const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(30);

/// Represents a single proving job with task assignment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QJob {
    pub job_id: QProvingJobDataID,
    pub task_id: TaskId,
    #[serde(default)]
    pub msg_id: String,
}

impl QJob {
    /// Create a new QJob with the given job_id and task_id
    pub fn new(job_id: QProvingJobDataID, task_id: TaskId) -> Self {
        Self {
            job_id,
            task_id,
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

    /// Create multiple jobs for the same task
    pub fn batch_new(job_ids: Vec<QProvingJobDataID>, task_id: TaskId) -> Vec<Self> {
        job_ids.into_iter()
            .map(|job_id| Self::new(job_id, task_id.clone()))
            .collect()
    }
}

// Display implementation for better logging
impl std::fmt::Display for QJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Job({:?}@{})", self.job_id, self.task_id)
    }
}


/// Job task store implementation with Redis backend
pub struct JobTaskStoreImpl {
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    rsmq: Arc<RsmqQueue>,
}

impl JobTaskStoreImpl {
    pub async fn new(redis_url: &str, pool_size: usize) -> Result<Self> {
        debug!("Initializing JobTaskStore with pool size {}", pool_size);

        let redis_pool = Arc::new(
            new_redis_async_pool(redis_url, pool_size)
                .await
                .context("Failed to create Redis pool")?
        );

        // Use the unified RsmqQueue instead of RsmqTaskQueue
        let rsmq = Arc::new(
            RsmqQueue::new(redis_url, pool_size, "job_task_store")
                .await
                .context("Failed to create RSMQ queue")?
        );

        Ok(Self { redis_pool, rsmq })
    }

    /// Generate queue name for a task
    #[inline]
    fn task_queue_name(task_id: &TaskId) -> String {
        format!("{}{}", TASK_RSMQ_PREFIX, task_id)
    }

    /// Create QueueId for a task
    #[inline]
    fn task_queue_id(task_id: &TaskId) -> QueueId {
        QueueId::WorkerEvent {
            queue_biz_key: Self::task_queue_name(task_id),
        }
    }

    /// Remove task from Redis list
    async fn remove_task_from_list(&self, task_id: &TaskId) -> Result<bool> {
        let mut conn = self.redis_pool.get().await?;
        let serialized = task_id.to_vec()?;
        let removed: i32 = conn.lrem(TASK_LIST_KEY, 0, &serialized).await?;

        if removed == 0 {
            warn!("Task '{}' not found in list", task_id);
        }

        Ok(removed > 0)
    }

    /// Get queue statistics for monitoring
    pub async fn get_queue_stats(&self) -> Result<HashMap<TaskId, u64>> {
        let task_ids = self.get_task_list().await?;
        let mut stats = HashMap::with_capacity(task_ids.len());

        for task_id in task_ids {
            if let Ok(count) = self.count_pending_jobs_in_task(&task_id).await {
                stats.insert(task_id, count);
            }
        }

        Ok(stats)
    }

    /// Get system status summary
    pub async fn get_system_status(&self) -> Result<String> {
        let tasks = self.get_task_list().await?;
        let stats = self.get_queue_stats().await?;

        let mut status = format!("Active Tasks: {}\n", tasks.len());
        for task_id in tasks {
            let count = stats.get(&task_id).copied().unwrap_or(0);
            status.push_str(&format!("  {}: {} pending\n", task_id, count));
        }

        Ok(status)
    }

    /// Clear all tasks and queues (use with caution)
    pub async fn clear_all(&self) -> Result<()> {
        let tasks = self.get_task_list().await?;

        for task_id in tasks {
            let queue_id = Self::task_queue_id(&task_id);
            let _ = self.rsmq.delete_queue(&queue_id).await;
        }

        let mut conn = self.redis_pool.get().await?;
        conn.del(TASK_LIST_KEY).await?;

        info!("Cleared all tasks and queues");
        Ok(())
    }
}

#[async_trait]
pub trait JobTaskStore {
    async fn enqueue_jobs(&self, task_id: &TaskId, jobs: &[QJob]) -> Result<()>;
    async fn claim_job_from_current_task(&self) -> Result<Option<QJob>>;
    async fn count_pending_jobs_in_current_task(&self) -> Result<u64>;
    async fn count_pending_jobs_in_task(&self, task_id: &TaskId) -> Result<u64>;
    async fn acknowledge_job_completion(&self, job: &QJob) -> Result<()>;
    async fn peek_current_task(&self) -> Result<Option<TaskId>>;
    async fn save_task_topology(&self, tasks: Vec<&JobsTask>) -> Result<()>;
    async fn get_task_list(&self) -> Result<Vec<TaskId>>;
    async fn save_job_dependency_graph(&self, graph: &JobsTaskGraph) -> Result<()>;
    async fn load_job_dependency_graph(&self) -> Result<JobsTaskGraph>;
}

#[async_trait]
impl JobTaskStore for JobTaskStoreImpl {
    async fn enqueue_jobs(&self, task_id: &TaskId, jobs: &[QJob]) -> Result<()> {
        if jobs.is_empty() {
            return Ok(());
        }

        debug!("Enqueuing {} jobs for task '{}'", jobs.len(), task_id);
        let queue_id = Self::task_queue_id(task_id);

        self.rsmq.create_queue_if_not_exists(&queue_id).await?;

        // Batch process jobs
        let results = futures::future::join_all(
            jobs.iter().map(|job| {
                let queue_id = queue_id.clone();
                let rsmq = self.rsmq.clone();
                async move {
                    let data = job.to_bytes()?;
                    rsmq.send_message(&queue_id, data).await
                }
            })
        ).await;

        // Check for errors
        let errors: Vec<_> = results.iter()
            .enumerate()
            .filter_map(|(i, r)| r.as_ref().err().map(|e| (i, e)))
            .collect();

        if !errors.is_empty() {
            return Err(anyhow!(
                "Failed to enqueue {} of {} jobs",
                errors.len(),
                jobs.len()
            ));
        }

        Ok(())
    }

    async fn claim_job_from_current_task(&self) -> Result<Option<QJob>> {
        let task_id = match self.peek_current_task().await? {
            Some(id) => id,
            None => return Ok(None),
        };

        let queue_id = Self::task_queue_id(&task_id);

        match self.rsmq.receive_message_with_id(&queue_id, Some(VISIBILITY_TIMEOUT)).await? {
            Some(msg) => {
                let job = QJob::from_bytes(&msg.message)?;
                debug!("Claimed {}", job);
                Ok(Some(job.with_msg_id(msg.id)))
            }
            None => Ok(None)
        }
    }

    async fn count_pending_jobs_in_current_task(&self) -> Result<u64> {
        let task_id = self.peek_current_task().await?
            .ok_or_else(|| anyhow!("No current task"))?;
        self.count_pending_jobs_in_task(&task_id).await
    }

    async fn count_pending_jobs_in_task(&self, task_id: &TaskId) -> Result<u64> {
        let queue_id = Self::task_queue_id(task_id);
        self.rsmq
            .get_queue_length(&queue_id)
            .await
            .context("Failed to get queue length")
    }

    async fn acknowledge_job_completion(&self, job: &QJob) -> Result<()> {
        debug!("Acknowledging {}", job);

        let queue_id = Self::task_queue_id(&job.task_id);

        // Delete message from queue
        self.rsmq.delete_message(&queue_id, &job.msg_id).await?;

        // Check if task is complete
        let remaining = self.rsmq.get_queue_length(&queue_id).await?;

        if remaining == 0 {
            info!("Task '{}' completed", job.task_id);

            if self.remove_task_from_list(&job.task_id).await? {
                // Best effort queue cleanup
                let _ = self.rsmq.delete_queue(&queue_id).await;
            }
        }

        Ok(())
    }

    async fn peek_current_task(&self) -> Result<Option<TaskId>> {
        let mut conn = self.redis_pool.get().await?;

        match conn.lindex::<_, Option<Vec<u8>>>(TASK_LIST_KEY, 0).await? {
            Some(bytes) => Ok(Some(TaskId::from_slice(&bytes)?)),
            None => Ok(None)
        }
    }

    async fn save_task_topology(&self, tasks: Vec<&JobsTask>) -> Result<()> {
        debug!("Saving {} tasks", tasks.len());

        let mut conn = self.redis_pool.get().await?;

        // Verify clean state
        let existing: usize = conn.llen(TASK_LIST_KEY).await?;
        if existing > 0 {
            return Err(anyhow!(
                "Cannot save: {} existing tasks. Clear first.",
                existing
            ));
        }

        // Process each task
        for task in tasks {
            if task.job_ids.is_empty() {
                warn!("Skipping empty task '{}'", task.task_id);
                continue;
            }

            // Create jobs using batch method
            let jobs = QJob::batch_new(
                task.job_ids.clone(),
                task.task_id.clone()
            );

            self.enqueue_jobs(&task.task_id, &jobs).await?;

            // Add to task list
            let serialized = task.task_id.to_vec()?;
            conn.rpush(TASK_LIST_KEY, serialized).await?;
        }

        info!("Topology saved successfully");
        Ok(())
    }

    async fn get_task_list(&self) -> Result<Vec<TaskId>> {
        let mut conn = self.redis_pool.get().await?;
        let tasks_raw: Vec<Vec<u8>> = conn.lrange(TASK_LIST_KEY, 0, -1).await?;

        let mut result = Vec::with_capacity(tasks_raw.len());
        for bytes in tasks_raw {
            result.push(TaskId::from_slice(&bytes)?);
        }
        Ok(result)
    }

    async fn save_job_dependency_graph(&self, graph: &JobsTaskGraph) -> Result<()> {
        let serialized = bincode::serialize(graph)?;

        let mut conn = self.redis_pool.get().await?;
        conn.set(JOB_GRAPH_KEY, serialized).await?;

        debug!("Job graph saved");
        Ok(())
    }

    async fn load_job_dependency_graph(&self) -> Result<JobsTaskGraph> {
        let mut conn = self.redis_pool.get().await?;
        let graph_bytes: Vec<u8> = conn.get(JOB_GRAPH_KEY).await?;

        bincode::deserialize::<JobsTaskGraph>(&graph_bytes)
            .context("Failed to deserialize job graph")
    }
}

// Additional helper methods for batch operations
impl JobTaskStoreImpl {
    /// Enqueue jobs by job IDs for a specific task
    pub async fn enqueue_job_ids(&self, job_ids: Vec<QProvingJobDataID>, task_id: TaskId) -> Result<()> {
        let jobs = QJob::batch_new(job_ids, task_id.clone());
        self.enqueue_jobs(&task_id, &jobs).await
    }

    /// Get detailed stats for a specific task
    pub async fn get_task_details(&self, task_id: &TaskId) -> Result<(u64, String)> {
        let count = self.count_pending_jobs_in_task(task_id).await?;
        let queue_name = Self::task_queue_name(task_id);
        Ok((count, queue_name))
    }

    /// Check if any tasks are pending
    pub async fn has_pending_tasks(&self) -> Result<bool> {
        Ok(self.peek_current_task().await?.is_some())
    }

    /// Get queue statistics using the unified API
    pub async fn get_detailed_queue_stats(&self, task_id: &TaskId) -> Result<QueueStats> {
        let queue_id = Self::task_queue_id(task_id);
        self.rsmq.get_queue_stats(&queue_id).await
    }
}