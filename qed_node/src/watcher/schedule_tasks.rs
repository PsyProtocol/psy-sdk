use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use qed_core::job::id::QProvingJobDataID;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn, debug};

use crate::watcher::watcher_service::current_timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub task_id: String,
    pub task_type: TaskType,
    pub execute_at: ExecutionTrigger,
    pub payload: serde_json::Value,
    pub created_at: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    DeleteProof,
    DeleteWitness,
    CleanupJob,
    CleanupJobStatus,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionTrigger {
    AtTimestamp(u64),
    AfterBlocks { target_height: u64 },
}

#[derive(Debug, Default, Clone)]
pub struct TaskStats {
    pub total_tasks: usize,
    pub proof_deletions: usize,
    pub witness_deletions: usize,
    pub job_cleanups: usize,
    pub job_status_cleanups: usize,
    pub custom_tasks: usize,
}

pub struct ScheduledTaskManager {
    tasks: Arc<RwLock<BTreeMap<String, ScheduledTask>>>,
    redis_pool: Arc<Pool<RedisConnectionManager>>,
    node_id: String,
}

impl ScheduledTaskManager {
    const REDIS_TTL: u64 = 7 * 24 * 3600; // 7 days
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECONDS: u64 = 60;
    const RETRY_DELAY_BLOCKS: u64 = 5;

    pub async fn new(
        redis_pool: Arc<Pool<RedisConnectionManager>>,
        node_id: String,
    ) -> Result<Self> {
        let manager = Self {
            tasks: Arc::new(RwLock::new(BTreeMap::new())),
            redis_pool,
            node_id,
        };

        manager.restore_from_redis().await?;
        Ok(manager)
    }

    pub async fn schedule_task(&self, task: ScheduledTask) -> Result<()> {
        let task_id = task.task_id.clone();

        self.tasks.write().await.insert(task_id.clone(), task.clone());
        self.persist_to_redis(&task).await?;

        debug!("Scheduled task {} with trigger {:?}", task_id, task.execute_at);
        Ok(())
    }

    /// Schedules a job status cleanup task to execute after specified blocks
    pub async fn schedule_job_status_cleanup(
        &self,
        job_id: QProvingJobDataID,
        blocks_to_wait: u64,
        current_height: u64,
    ) -> Result<()> {
        let target_height = current_height + blocks_to_wait;

        let task = ScheduledTask {
            task_id: format!("cleanup_job_status_{}", job_id.to_hex_string()),
            task_type: TaskType::CleanupJobStatus,
            execute_at: ExecutionTrigger::AfterBlocks { target_height },
            payload: serde_json::json!({
                "job_id": job_id,
                "scheduled_at_height": current_height,
                "target_height": target_height,
            }),
            created_at: current_timestamp(),
            retry_count: 0,
        };

        self.schedule_task(task).await?;
        debug!(
            "Scheduled job status cleanup for job {:?} at block height {} (current: {})",
            job_id, target_height, current_height
        );
        Ok(())
    }

    pub async fn get_ready_tasks(&self, current_height: u64, current_time: u64) -> Result<Vec<ScheduledTask>> {
        let tasks = self.tasks.read().await;

        Ok(tasks.values()
            .filter(|task| self.is_task_ready(task, current_height, current_time))
            .cloned()
            .collect())
    }

    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        self.remove_task(task_id).await
    }

    pub async fn retry_task(&self, mut task: ScheduledTask) -> Result<()> {
        task.retry_count += 1;

        if task.retry_count >= Self::MAX_RETRIES {
            error!("Task {} failed after {} retries, removing", task.task_id, Self::MAX_RETRIES);
            return self.remove_task(&task.task_id).await;
        }

        // Apply retry delay
        task.execute_at = match task.execute_at {
            ExecutionTrigger::AtTimestamp(ts) =>
                ExecutionTrigger::AtTimestamp(ts + Self::RETRY_DELAY_SECONDS),
            ExecutionTrigger::AfterBlocks { target_height } =>
                ExecutionTrigger::AfterBlocks { target_height: target_height + Self::RETRY_DELAY_BLOCKS },
        };

        self.update_task(task).await
    }

    pub async fn list_tasks(&self) -> Vec<ScheduledTask> {
        self.tasks.read().await.values().cloned().collect()
    }

    pub async fn get_task_stats(&self) -> TaskStats {
        let tasks = self.tasks.read().await;
        let mut stats = TaskStats::default();

        stats.total_tasks = tasks.len();

        for task in tasks.values() {
            match task.task_type {
                TaskType::DeleteProof => stats.proof_deletions += 1,
                TaskType::DeleteWitness => stats.witness_deletions += 1,
                TaskType::CleanupJob => stats.job_cleanups += 1,
                TaskType::CleanupJobStatus => stats.job_status_cleanups += 1,
                TaskType::Custom(_) => stats.custom_tasks += 1,
            }
        }

        stats
    }

    fn is_task_ready(&self, task: &ScheduledTask, current_height: u64, current_time: u64) -> bool {
        match task.execute_at {
            ExecutionTrigger::AtTimestamp(timestamp) => current_time >= timestamp,
            ExecutionTrigger::AfterBlocks { target_height } => current_height >= target_height,
        }
    }

    async fn update_task(&self, task: ScheduledTask) -> Result<()> {
        let task_id = task.task_id.clone();

        self.tasks.write().await.insert(task_id, task.clone());
        self.persist_to_redis(&task).await
    }

    async fn remove_task(&self, task_id: &str) -> Result<()> {
        self.tasks.write().await.remove(task_id);

        let key = self.redis_key(task_id);
        self.redis_pool.get().await?.del(&key).await?;

        Ok(())
    }

    async fn persist_to_redis(&self, task: &ScheduledTask) -> Result<()> {
        let key = self.redis_key(&task.task_id);
        let value = serde_json::to_string(task)?;

        self.redis_pool.get().await?
            .set_ex(&key, value, Self::REDIS_TTL)
            .await?;

        Ok(())
    }

    async fn restore_from_redis(&self) -> Result<()> {
        let pattern = format!("watcher:scheduled:{}:*", self.node_id);
        let mut conn = self.redis_pool.get().await?;

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut *conn)
            .await?;

        let mut tasks = self.tasks.write().await;
        let mut restored_count = 0;

        for key in keys {
            if let Ok(value) = conn.get::<_, String>(&key).await {
                if let Ok(task) = serde_json::from_str::<ScheduledTask>(&value) {
                    tasks.insert(task.task_id.clone(), task);
                    restored_count += 1;
                }
            }
        }

        debug!("Restored {} scheduled tasks from Redis", restored_count);
        Ok(())
    }

    fn redis_key(&self, task_id: &str) -> String {
        format!("watcher:scheduled:{}:{}", self.node_id, task_id)
    }
}
