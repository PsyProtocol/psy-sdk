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
use crate::queue::{new_redis_async_pool};
use crate::queue::rsmq_task_queue::RsmqTaskQueue;

// Constants
const JOB_GRAPH_KEY: &str = "job_graph:current";
const TASK_LIST_KEY: &str = "task:list";
const TASK_RSMQ_QUEUE_PREFIX: &str = "task:rsmq:";
const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(180);


/// Represents a single job in the proving system
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QJob<T = String> {
    pub job_id: QProvingJobDataID,
    pub task_id: TaskId,
    pub parent: Option<QProvingJobDataID>,
    #[serde(default)]
    pub msg_id: T,
}

impl<T: Default> QJob<T> {
    pub fn new(job_id: QProvingJobDataID, task_id: TaskId) -> Self {
        Self {
            job_id,
            task_id,
            parent: None,
            msg_id: T::default(),
        }
    }
    
    pub fn new_with_parent(job_id: QProvingJobDataID, task_id: TaskId, parent: QProvingJobDataID) -> Self {
        Self {
            job_id,
            task_id,
            parent: Some(parent),
            msg_id: T::default(),
        }
    }
}

impl QJob<String> {
    fn with_msg_id(mut self, msg_id: String) -> Self {
        self.msg_id = msg_id;
        self
    }
}

/// Generates the RSMQ queue name for a given task ID
#[inline]
fn task_queue_name(task_id: &TaskId) -> String {
    format!("{}{}", TASK_RSMQ_QUEUE_PREFIX, task_id)
}

/// Main implementation of the JobTaskStore
pub struct JobTaskStoreImpl {
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    rsmq: Arc<RsmqTaskQueue>,
}

impl JobTaskStoreImpl {
    /// Creates a new JobTaskStore instance
    pub async fn new(redis_url: &str, pool_size: usize) -> Result<Self> {
        info!("Initializing JobTaskStore - URL: {}, Pool size: {}", redis_url, pool_size);

        let redis_pool = Arc::new(
            new_redis_async_pool(redis_url, pool_size)
                .await
                .context("Failed to create Redis pool")?
        );

        let rsmq = Arc::new(
            RsmqTaskQueue::new(redis_url, pool_size)
                .await
                .context("Failed to create RsmqTaskQueue")?
        );

        info!("JobTaskStore initialized successfully");
        Ok(Self { redis_pool, rsmq })
    }

    /// Removes a task from the Redis list
    async fn remove_task_from_list(&self, task_id: &TaskId) -> Result<bool> {
        let mut conn = self.redis_pool.get().await?;
        let serialized = bincode::serialize(task_id)?;
        let removed: i32 = conn.lrem(TASK_LIST_KEY, 0, &serialized).await?;

        if removed > 0 {
            info!("Removed {} occurrences of task '{:?}' from task list", removed, task_id);
        } else {
            warn!("Task '{:?}' not found in task list", task_id);
        }

        Ok(removed > 0)
    }

    /// Serializes and deserializes task IDs
    #[inline]
    fn serialize_task(task_id: &TaskId) -> Result<Vec<u8>> {
        bincode::serialize(task_id).context("Failed to serialize task ID")
    }

    #[inline]
    fn deserialize_task(bytes: &[u8]) -> Result<TaskId> {
        bincode::deserialize(bytes).context("Failed to deserialize task ID")
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
            warn!("No jobs to enqueue");
            return Ok(());
        }

        info!("Enqueuing {} jobs for task '{}'", jobs.len(), task_id);
        let queue = task_queue_name(task_id);

        self.rsmq.create_queue_if_not_exists(&queue).await?;

        // Process jobs concurrently
        let errors: Vec<_> = futures::future::join_all(
            jobs.iter().enumerate().map(|(idx, job)| {
                let queue = queue.clone();
                let rsmq = self.rsmq.clone();
                let job_id = job.job_id.clone();

                async move {
                    match bincode::serialize(job) {
                        Ok(data) => {
                            rsmq.send_message(&queue, data)
                                .await
                                .map_err(|e| (idx, job_id, anyhow!("Send failed: {}", e)))
                        }
                        Err(e) => Err((idx, job_id, anyhow!("Serialization failed: {}", e)))
                    }
                }
            })
        )
        .await
        .into_iter()
        .filter_map(|r| r.err())
        .collect();

        if !errors.is_empty() {
            for (idx, job_id, err) in &errors {
                error!("Job {} ({:?}): {}", idx, job_id, err);
            }
            return Err(anyhow!("Failed to enqueue {} out of {} jobs", errors.len(), jobs.len()));
        }

        info!("Successfully enqueued all {} jobs", jobs.len());
        Ok(())
    }

    async fn claim_job_from_current_task(&self) -> Result<Option<QJob>> {
        let task_id = match self.peek_current_task().await? {
            Some(id) => id,
            None => {
                debug!("No current task available");
                return Ok(None);
            }
        };

        let queue = task_queue_name(&task_id);
        debug!("Claiming job from task '{}' (queue: '{}')", task_id, queue);

        match self.rsmq.receive_message_with_id(&queue, Some(VISIBILITY_TIMEOUT)).await? {
            Some(msg) => {
                let job: QJob = bincode::deserialize(&msg.message)
                    .context("Failed to deserialize job")?;

                info!("Claimed job {:?} from task '{}'", job.job_id, task_id);
                Ok(Some(job.with_msg_id(msg.id)))
            }
            None => {
                debug!("No jobs available in task '{}'", task_id);
                Ok(None)
            }
        }
    }

    async fn count_pending_jobs_in_current_task(&self) -> Result<u64> {
        let task_id = self.peek_current_task().await?
            .ok_or_else(|| anyhow!("No current task available"))?;
        self.count_pending_jobs_in_task(&task_id).await
    }

    async fn count_pending_jobs_in_task(&self, task_id: &TaskId) -> Result<u64> {
        let count = self.rsmq
            .get_queue_length(&task_queue_name(task_id))
            .await
            .with_context(|| format!("Failed to count jobs for task '{:?}'", task_id))?;

        debug!("Task {:?} has {} pending jobs", task_id, count);
        Ok(count)
    }

    async fn acknowledge_job_completion(&self, job: &QJob) -> Result<()> {
        info!("Acknowledging job {:?} completion", job.job_id);

        let queue = task_queue_name(&job.task_id);

        // Delete message from queue
        self.rsmq
            .delete_message(&queue, &job.msg_id)
            .await
            .with_context(|| format!("Failed to delete message for job {:?}", job.job_id))?;

        // Check if task is complete
        let remaining = self.rsmq.get_queue_length(&queue).await?;
        info!("Task '{:?}' has {} remaining jobs", job.task_id, remaining);

        if remaining == 0 {
            info!("Task '{:?}' completed - cleaning up", job.task_id);

            if self.remove_task_from_list(&job.task_id).await? {
                if let Err(e) = self.rsmq.delete_queue(&queue).await {
                    error!("Failed to delete queue '{}': {}", queue, e);
                }
            }
        }

        Ok(())
    }

    async fn peek_current_task(&self) -> Result<Option<TaskId>> {
        trace!("Peeking at current task");

        let mut conn = self.redis_pool.get().await?;

        conn.lindex::<_, Option<Vec<u8>>>(TASK_LIST_KEY, 0)
            .await?
            .map(|bytes| Self::deserialize_task(&bytes))
            .transpose()
    }
    async fn save_task_topology(&self, tasks: Vec<&JobsTask>) -> Result<()> {
        info!("Saving topology with {} tasks", tasks.len());

        let mut conn = self.redis_pool.get().await?;

        // Verify list is empty
        let existing: usize = conn.llen(TASK_LIST_KEY).await?;
        if existing > 0 {
            return Err(anyhow!("Cannot save topology: {} existing tasks found. Clear existing tasks first.", existing));
        }

        // Process tasks
        for (index, task) in tasks.iter().enumerate() {
            trace!("Processing task {}/{}: '{:?}' with {} jobs", index + 1, tasks.len(), task.task_id, task.job_ids.len());

            // Skip empty tasks
            if task.job_ids.is_empty() {
                warn!("Task '{:?}' has no jobs to enqueue", task.task_id);
                continue;
            }

            // 1. Create jobs
            let jobs: Vec<_> = task.job_ids
                .iter()
                .map(|job_id| QJob::new(job_id.clone(), task.task_id.clone()))
                .collect();

            // 2. Enqueue jobs FIRST (before adding task to list)
            self.enqueue_jobs(&task.task_id, &jobs)
                .await
                .with_context(|| format!("Failed to enqueue {} jobs for task '{:?}'",
                    jobs.len(), task.task_id
                ))?;

            info!("Enqueued {} jobs for task '{:?}'", jobs.len(), task.task_id);

            // 3. Add task to list ONLY AFTER jobs are successfully enqueued
            let serialized = Self::serialize_task(&task.task_id)?;
            conn.rpush(TASK_LIST_KEY, serialized)
                .await
                .with_context(|| format!("Failed to save task '{:?}' to list", task.task_id))?;

            debug!("Added task '{:?}' to Redis list", task.task_id);
        }

        info!("Successfully saved topology");
        Ok(())
    }

    async fn get_task_list(&self) -> Result<Vec<TaskId>> {
        debug!("Retrieving complete task list");

        let mut conn = self.redis_pool.get().await?;
        let tasks_raw: Vec<Vec<u8>> = conn.lrange(TASK_LIST_KEY, 0, -1).await?;

        debug!("Retrieved {} raw task entries", tasks_raw.len());

        let mut task_ids = Vec::with_capacity(tasks_raw.len());
        let mut failed_count = 0;

        for task_bytes in tasks_raw {
            match bincode::deserialize::<TaskId>(&task_bytes) {
                Ok(task_id) => {
                    trace!("Deserialized task ID: '{:?}'", task_id);
                    task_ids.push(task_id);
                },
                Err(e) => {
                    error!("Failed to deserialize task ID: {}", e);
                    failed_count += 1;
                }
            }
        }

        if failed_count > 0 {
            warn!("Failed to deserialize {} task IDs", failed_count);
        }

        info!("Retrieved {} task IDs successfully", task_ids.len());
        Ok(task_ids)
    }

    async fn save_job_dependency_graph(&self, graph: &JobsTaskGraph) -> Result<()> {
        info!("Saving job dependency graph");

        let serialized_graph = bincode::serialize(graph)
            .map_err(|e| {
                error!("Failed to serialize job graph: {}", e);
                e
            })?;

        debug!("Serialized graph size: {} bytes", serialized_graph.len());

        let mut conn = self.redis_pool.get().await?;
        conn.set(JOB_GRAPH_KEY, serialized_graph).await
            .map_err(|e| {
                error!("Failed to store job graph in Redis: {}", e);
                e
            })?;

        info!("Successfully saved job dependency graph");
        Ok(())
    }

    async fn load_job_dependency_graph(&self) -> Result<JobsTaskGraph> {
        info!("Loading job dependency graph");

        let mut conn = self.redis_pool.get().await?;
        let graph_bytes: Vec<u8> = conn.get(JOB_GRAPH_KEY).await
            .map_err(|e| {
                error!("Failed to retrieve job graph from Redis: {}", e);
                e
            })?;

        debug!("Retrieved graph data: {} bytes", graph_bytes.len());

        let graph = bincode::deserialize::<JobsTaskGraph>(&graph_bytes)
            .map_err(|e| {
                error!("Failed to deserialize job graph: {}", e);
                e
            })?;

        info!("Successfully loaded job dependency graph");
        Ok(graph)
    }
}

// Utility methods
impl JobTaskStoreImpl {
    /// Gets queue statistics for all tasks
    pub async fn get_queue_stats(&self) -> Result<HashMap<TaskId, u64>> {
        let task_ids = self.get_task_list().await?;
        let mut stats = HashMap::with_capacity(task_ids.len());

        for task_id in task_ids {
            match self.count_pending_jobs_in_task(&task_id).await {
                Ok(count) => { stats.insert(task_id, count); }
                Err(e) => error!("Failed to get stats for task '{:?}': {}", task_id, e),
            }
        }

        Ok(stats)
    }

    /// Gets system status summary
    pub async fn get_system_status(&self) -> Result<String> {
        let tasks = self.get_task_list().await?;
        let stats = self.get_queue_stats().await?;

        let mut status = format!("=== JobTaskStore Status ===\nActive Tasks: {}\n\n", tasks.len());

        for task_id in tasks {
            let count = stats.get(&task_id).copied().unwrap_or(0);
            status.push_str(&format!("Task '{:?}': {} pending jobs\n", task_id, count));
        }

        Ok(status)
    }
}
