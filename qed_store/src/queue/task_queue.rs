use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use rsmq::PooledRsmq;
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug, trace};
use qed_core::job::id::{JobsTask, JobsTaskGraph, QProvingJobDataID, TaskId};
use crate::queue::{new_redis_async_pool, new_rsmq_pool, QueueId, RsmqQueue};
use crate::queue::rsmq_task_queue::RsmqTaskQueue;
// ============================================================================
// Constants
// ============================================================================

/// Redis key for storing the current job dependency graph
const JOB_GRAPH_KEY: &str = "job_graph:current";

/// Redis key for the task topology list (ordered list of task IDs)
const TASK_LIST_KEY: &str = "task:list";

/// Prefix for RSMQ queue names
const TASK_RSMQ_QUEUE_PREFIX: &str = "task:rsmq:";
/// How long a job remains invisible to other workers after being claimed
const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(180);

/// Prefix for job ID keys in Redis
const JOB_ID_PREFIX: &str = "jobid:";

// ============================================================================
// Data Structures
// ============================================================================


/// Represents a single job in the proving system
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QJob {
    /// Unique identifier for this proving job
    pub job_id: QProvingJobDataID,
    /// ID of the task this job belongs to
    pub task_id: TaskId,
    /// RSMQ message ID needed for acknowledging/deleting the message
    pub msg_id: String,
}

impl QJob {
    /// Creates a new Job instance with auto-generated Redis key
    pub fn new(
        job_id: QProvingJobDataID,
        task_id: TaskId,
    ) -> Self {
        Self {
            job_id,
            task_id,
            msg_id: "".to_string(),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generates the RSMQ queue name for a given task ID
pub fn task_rsmq_queue_name(task_id: &TaskId) -> String {
    format!("{}{}", TASK_RSMQ_QUEUE_PREFIX, task_id)
}

// ============================================================================
// Main Implementation
// ============================================================================

/// Main implementation of the JobTaskStore with connection pooling
pub struct JobTaskStoreImpl {
    /// BB8 Redis pool for general Redis operations
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    /// RsmqTaskQueue for queue operations
    rsmq: Arc<RsmqTaskQueue>,
}

impl JobTaskStoreImpl {
    /// Creates a new JobTaskStore instance with the specified configuration
    pub async fn new(
        redis_url: &str,
        pool_size: usize,
    ) -> Result<Self> {
        info!(
            "Initializing JobTaskStore - URL: {}, Pool size: {}",
            redis_url, pool_size,
        );

        // Create Redis pool for general operations
        let redis_pool = new_redis_async_pool(redis_url, pool_size).await
            .map_err(|e| {
                error!("Failed to create Redis pool: {}", e);
                e
            })?;

        // Create RsmqTaskQueue
        let rsmq = RsmqTaskQueue::new(redis_url, pool_size).await?;

        info!("JobTaskStore initialized successfully");

        Ok(Self {
            redis_pool: Arc::new(redis_pool),
            rsmq: Arc::new(rsmq),
        })
    }



    /// Removes a specific task from the Redis list by value
    async fn remove_task_from_list(&self, task_id: &TaskId) -> Result<bool> {
        let mut conn = self.redis_pool.get().await?;

        // Serialize the task ID to match what's stored
        let serialized_task_id = bincode::serialize(task_id)?;

        // Remove all occurrences of this task ID from the list
        // LREM returns the number of elements removed
        let removed_count: i32 = conn.lrem(TASK_LIST_KEY, 0, &serialized_task_id).await?;

        if removed_count > 0 {
            info!("Removed {} occurrences of task '{:?}' from task list", removed_count, task_id);
            Ok(true)
        } else {
            warn!("Task '{:?}' not found in task list", task_id);
            Ok(false)
        }
    }
}

// ============================================================================
// JobTaskStore Trait Implementation
// ============================================================================

#[async_trait::async_trait]
pub trait JobTaskStore {
    /// Registers a new task: stores task metadata and initializes its RSMQ queue
    async fn register_task(&self, task_id: &TaskId) -> Result<()>;

    /// Enqueues multiple jobs into a specific task's queue
    async fn enqueue_jobs(&self, task_id: &TaskId, jobs: &Vec<QJob>) -> Result<()>;

    /// Attempts to claim one job from the current task's queue
    async fn claim_job_from_current_task(&self) -> Result<Option<QJob>>;

    /// Returns the number of pending (visible) jobs in the specified task's queue
    async fn count_pending_jobs_in_current_task(&self) -> Result<u64>;

    /// Returns the number of pending (visible) jobs in the specified task's queue
    async fn count_pending_jobs_in_task(&self, task_id: &TaskId) -> Result<u64>;

    /// Acknowledges the successful completion of a job and removes it from the queue
    async fn acknowledge_job_completion(&self, job: &QJob) -> Result<()>;

    /// Returns the next executable task from the front of the task topology queue
    async fn peek_current_task(&self) -> Result<Option<TaskId>>;

    /// Stores the topologically sorted list of task IDs into Redis
    async fn save_task_topology(&self, task_ids: Vec<&JobsTask>) -> Result<()>;

    /// Gets all task IDs in the topology
    async fn get_task_list(&self) -> Result<Vec<TaskId>>;

    /// Stores the full job dependency graph (DAG)
    async fn save_job_dependency_graph(&self, graph: &JobsTaskGraph) -> Result<()>;

    /// Loads the complete job dependency graph (DAG)
    async fn load_job_dependency_graph(&self) -> Result<JobsTaskGraph>;
}

#[async_trait]
impl JobTaskStore for JobTaskStoreImpl {
    async fn register_task(&self, task_id: &TaskId) -> Result<()> {
        info!("Registering task: {}", task_id);

        let rsmq_queue = task_rsmq_queue_name(task_id);
        // Create RSMQ queue if it doesn't exist
        debug!("Creating RSMQ queue if not exists: {}", rsmq_queue);
        self.rsmq.create_queue_if_not_exists(&rsmq_queue).await
            .map_err(|e| {
                error!("Failed to create RSMQ queue '{}' for task '{}': {}", rsmq_queue, task_id, e);
                e
            })?;

        // Store task ID in Redis list
        let mut conn = self.redis_pool.get().await
            .map_err(|e| {
                error!("Failed to get Redis connection: {}", e);
                e
            })?;

        let serialized_task_id = bincode::serialize(task_id)
            .map_err(|e| {
                error!("Failed to serialize task ID '{}': {}", task_id, e);
                e
            })?;

        trace!("Serialized task ID size: {} bytes", serialized_task_id.len());

        conn.rpush(TASK_LIST_KEY, &serialized_task_id).await
            .map_err(|e| {
                error!("Failed to push task '{}' to Redis list: {}", task_id, e);
                e
            })?;

        info!("Successfully registered task '{}' with queue '{}'", task_id, rsmq_queue);
        Ok(())
    }

    async fn enqueue_jobs(&self, task_id: &TaskId, jobs: &Vec<QJob>) -> Result<()> {
        let jobs_count = jobs.len();
        info!("Enqueuing {} jobs for task '{}'", jobs_count, task_id);

        if jobs.is_empty() {
            warn!("No jobs to enqueue");
            return Ok(());
        }

        let rsmq_queue = task_rsmq_queue_name(task_id);

        // Ensure queue exists
        self.rsmq.create_queue_if_not_exists(&rsmq_queue).await?;

        // Serialize all jobs
        let mut serialized_jobs = Vec::with_capacity(jobs.len());
        for job in jobs {
            let msg = bincode::serialize(job)
                .map_err(|e| {
                    error!("Failed to serialize job {:?}: {}", job.job_id, e);
                    e
                })?;
            serialized_jobs.push(msg);
        }

        // Send jobs in batch
        let success_count = self.rsmq.send_messages_batch(&rsmq_queue, serialized_jobs).await?;

        info!("Successfully enqueued {}/{} jobs for task '{}'", success_count, jobs_count, task_id);
        Ok(())
    }

    async fn claim_job_from_current_task(&self) -> Result<Option<QJob>> {
        debug!("Attempting to claim a job from current task");

        // Get current task
        let task_id = self.peek_current_task().await?
            .ok_or_else(|| {
                debug!("No current task available for job claiming");
                anyhow::anyhow!("No current task available")
            })?;

        let queue_name = task_rsmq_queue_name(&task_id);
        debug!("Claiming job from task '{}' (queue: '{}')", task_id, queue_name);


        // Receive message with visibility timeout
        match self.rsmq.receive_message_with_id(&queue_name, Some(VISIBILITY_TIMEOUT)).await? {
            Some(rsmq_msg) => {
                let msg_id = rsmq_msg.id.clone();

                trace!("Received RSMQ message - ID: '{}', Size: {} bytes", msg_id, rsmq_msg.message.len());

                let job = bincode::deserialize::<QJob>(&rsmq_msg.message)
                    .map_err(|e| {
                        error!("Failed to deserialize job from RSMQ message: {}", e);
                        e
                    })?;

                info!(
                    "Successfully claimed job - ID: {:?}, Task: '{}', Message ID: '{}'",
                    job.job_id, job.task_id, msg_id
                );

                let job = QJob {
                    job_id: job.job_id,
                    task_id: job.task_id,
                    msg_id: msg_id.clone(),
                };
                Ok(Some(job))
            },
            None => {
                debug!("No jobs available in queue for task '{}'", task_id);
                Ok(None)
            }
        }
    }

    async fn count_pending_jobs_in_current_task(&self) -> Result<u64> {
        let task_id = self.peek_current_task().await?
            .ok_or_else(|| {
                debug!("No current task available for counting jobs");
                anyhow::anyhow!("No current task available")
            })?;
        debug!("Counting pending jobs for task '{}'", task_id);


        let queue_name = task_rsmq_queue_name(&task_id);
        let count = self.rsmq.count_queue_len(&queue_name).await
            .map_err(|e| {
                error!("Failed to count jobs for task '{}': {}", task_id, e);
                e
            })?;

        debug!("Task {} has {} pending jobs", task_id, count);
        Ok(count)
    }

    async fn count_pending_jobs_in_task(&self, task_id: &TaskId) -> Result<u64> {
        debug!("Counting pending jobs for task '{}'", task_id);


        let queue_name = task_rsmq_queue_name(task_id);
        let count = self.rsmq.count_queue_len(&queue_name).await
            .map_err(|e| {
                error!("Failed to count jobs for task '{}': {}", task_id, e);
                e
            })?;

        debug!("Task {:?} has {} pending jobs", task_id, count);
        Ok(count)
    }

    async fn acknowledge_job_completion(&self, job: &QJob) -> Result<()> {
        info!(
            "Acknowledging job completion - ID: {:?}, Task: '{:?}', Message ID: '{}'",
            job.job_id, job.task_id, job.msg_id
        );

        let queue_name = task_rsmq_queue_name(&job.task_id);

        // Delete the message from RSMQ
        self.rsmq.delete_message(&queue_name, &job.msg_id).await
            .map_err(|e| {
                error!(
                    "Failed to delete message '{}' for job {:?}: {}",
                    job.msg_id, job.job_id, e
                );
                e
            })?;

        debug!("Deleted RSMQ message '{}'", job.msg_id);

        // Check if task is complete
        let remaining_jobs = self.rsmq.count_queue_len(&queue_name).await?;
        info!("Task '{:?}' has {} remaining jobs", job.task_id, remaining_jobs);

        if remaining_jobs == 0 {
            info!("Task '{:?}' completed - removing from task list and deleting queue", job.task_id);

            // Remove task from Redis list (not using lpop to avoid ordering issues)
            let removed = self.remove_task_from_list(&job.task_id).await?;

            if removed {
                // Delete the RSMQ queue since the task is complete
                match self.rsmq.delete_queue(&queue_name).await {
                    Ok(_) => info!("Deleted RSMQ queue '{}' for completed task '{:?}'", queue_name, job.task_id),
                    Err(e) => error!("Failed to delete RSMQ queue '{}': {}", queue_name, e),
                }
            } else {
                warn!("Task '{:?}' was already removed from list", job.task_id);
            }
        }

        info!("Successfully acknowledged job completion: {:?}", job.job_id);
        Ok(())
    }

    async fn peek_current_task(&self) -> Result<Option<TaskId>> {
        trace!("Peeking at current task");

        let mut conn = self.redis_pool.get().await?;
        let task_bytes: Option<Vec<u8>> = conn.lindex(TASK_LIST_KEY, 0).await?;

        match task_bytes {
            Some(bytes) => {
                match bincode::deserialize::<TaskId>(&bytes) {
                    Ok(task_id) => {
                        debug!("Current task: '{:?}'", task_id);
                        Ok(Some(task_id))
                    },
                    Err(e) => {
                        error!("Failed to deserialize current task: {}", e);
                        Err(e.into())
                    }
                }
            },
            None => {
                debug!("No current task in the queue");
                Ok(None)
            }
        }
    }

    /// Saves task topology and generates jobs for each task
    ///
    /// This function:
    /// 1. Clears the existing task list
    /// 2. For each JobsTask:
    ///    - Saves the task_id to Redis list
    ///    - Creates the RSMQ queue for the task
    ///    - Generates Job instances for each job_id
    ///    - Enqueues all jobs to the task's RSMQ queue
    async fn save_task_topology(&self, tasks: Vec<&JobsTask>) -> Result<()> {
        info!("Saving task topology with {} tasks", tasks.len());

        let mut conn = self.redis_pool.get().await?;

        // Check if task list is empty before pushing new tasks
        let existing_task_count: usize = conn.llen(TASK_LIST_KEY).await
            .map_err(|e| {
                error!("Failed to check existing task list length: {}", e);
                e
            })?;
        debug!("Existing task count in Redis: {}", existing_task_count);
        if existing_task_count > 0 {
            // Get some task IDs for the error message
            let sample_tasks: Vec<Vec<u8>> = conn.lrange(TASK_LIST_KEY, 0, 4).await?;
            let mut task_ids = Vec::new();

            for task_bytes in sample_tasks.iter().take(3) {
                if let Ok(task_id) = bincode::deserialize::<TaskId>(task_bytes) {
                    task_ids.push(task_id);
                }
            }

            let sample_text = if task_ids.is_empty() {
                String::new()
            } else {
                format!(" Sample tasks: {:?}", task_ids)
            };

            warn!(
                "Cannot save task topology: task list is not empty! Found {} existing tasks.{} \
                Please complete or clear existing tasks before saving new topology.",
                existing_task_count, sample_text
            );
        }

        // Process each task
        for (index, job_task) in tasks.iter().enumerate() {
            trace!(
                "Processing task {}/{}: '{:?}' with {} jobs",
                index + 1,
                tasks.len(),
                job_task.task_id,
                job_task.job_ids.len()
            );

            // 1. Save task_id to Redis list
            let serialized_task_id = bincode::serialize(&job_task.task_id)?;
            conn.rpush(TASK_LIST_KEY, serialized_task_id).await
                .map_err(|e| {
                    error!("Failed to save task_id '{:?}' to list: {}", job_task.task_id, e);
                    e
                })?;

            debug!("Saved task_id '{:?}' to Redis list", job_task.task_id);

            // 2. Generate Job instances for each job_id
            let mut jobs = Vec::with_capacity(job_task.job_ids.len());

            for (job_index, job_id) in job_task.job_ids.iter().enumerate() {
                // Create job with empty data (you can modify this to include actual job data)
                let job = QJob::new(
                    job_id.clone(),
                    job_task.task_id.clone(),
                );

                trace!(
                    "Created job {}/{} for task '{:?}': {:?}",
                    job_index + 1,
                    job_task.job_ids.len(),
                    job_task.task_id,
                    job_id
                );

                jobs.push(job);
            }

            // 3. Enqueue all jobs to the task's RSMQ queue
            if !jobs.is_empty() {
                self.enqueue_jobs(&job_task.task_id, &jobs).await
                    .map_err(|e| {
                        error!(
                            "Failed to enqueue {} jobs for task '{:?}': {}",
                            jobs.len(), job_task.task_id, e
                        );
                        e
                    })?;

                info!(
                    "Enqueued {} jobs for task '{:?}' ",
                    jobs.len(), job_task.task_id,
                );
            } else {
                warn!("Task '{:?}' has no jobs to enqueue", job_task.task_id);
            }
        }

        info!(
            "Successfully saved task topology: {} tasks",
            tasks.len()
        );
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

// ============================================================================
// Additional Helper Methods
// ============================================================================

impl JobTaskStoreImpl {
    /// Gets queue statistics for all tasks
    pub async fn get_queue_stats(&self) -> Result<HashMap<TaskId, u64>> {
        info!("Collecting queue statistics for all tasks");

        let mut stats = HashMap::new();
        let task_ids = self.get_task_list().await?;

        debug!("Collecting stats for {} tasks", task_ids.len());

        for task_id in task_ids {
            let queue_name = task_rsmq_queue_name(&task_id);
            match self.rsmq.count_queue_len(&queue_name).await {
                Ok(count) => {
                    trace!("Task '{:?}' has {} pending jobs", task_id, count);
                    stats.insert(task_id, count);
                },
                Err(e) => {
                    error!("Failed to get queue stats for task '{:?}': {}", task_id, e);
                }
            }
        }

        info!("Collected stats for {} tasks", stats.len());
        Ok(stats)
    }

    /// Clears all data from Redis (use with caution!)
    pub async fn clear_all(&self) -> Result<()> {
        warn!("Clearing all JobTaskStore data from Redis!");

        // Get all task IDs before clearing
        let task_ids = self.get_task_list().await?;

        // Delete all RSMQ queues
        for task_id in &task_ids {
            let queue_name = task_rsmq_queue_name(task_id);
            match self.rsmq.delete_queue(&queue_name).await {
                Ok(_) => debug!("Deleted queue '{}'", queue_name),
                Err(e) => error!("Failed to delete queue '{}': {}", queue_name, e),
            }
        }

        // Clear Redis data
        let mut conn = self.redis_pool.get().await?;

        let keys_deleted: Vec<i32> = redis::pipe()
            .del(TASK_LIST_KEY)
            .del(JOB_GRAPH_KEY)
            .query_async(&mut *conn)
            .await?;

        info!("Deleted {} keys from Redis", keys_deleted.len());
        warn!("All JobTaskStore data cleared");
        Ok(())
    }

    /// Gets detailed system status
    pub async fn get_system_status(&self) -> Result<String> {
        let task_ids = self.get_task_list().await?;
        let stats = self.get_queue_stats().await?;

        let mut status = format!(
            "=== JobTaskStore System Status ===\n\
             Active Tasks: {}\n\n",
            task_ids.len()
        );

        for task_id in task_ids {
            let job_count = stats.get(&task_id).unwrap_or(&0);
            let queue_name = task_rsmq_queue_name(&task_id);
            status.push_str(&format!(
                "Task: '{:?}' (Queue: '{}') - {} pending jobs\n",
                task_id, queue_name, job_count
            ));
        }

        Ok(status)
    }
}
