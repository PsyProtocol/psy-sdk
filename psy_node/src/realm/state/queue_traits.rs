use anyhow::Result;
use async_trait::async_trait;
use plonky2::hash::hash_types::RichField;
use psy_store::queue::{QueueId, QueueStats};

use crate::common_v2::traits::realm::{RealmEdgeUserUpdateSubmission, UniqueQueueId};

#[async_trait]
pub trait EdgeSubmissionQueue<F: RichField>: Send + Sync {
    async fn enqueue(&self, checkpoint: UniqueQueueId, submission: RealmEdgeUserUpdateSubmission<F>) -> Result<()>;
    async fn enqueue_batch(&self, checkpoint: UniqueQueueId, submissions: Vec<RealmEdgeUserUpdateSubmission<F>>) -> Result<()>;
    async fn drain_all(&self, checkpoint: UniqueQueueId) -> Result<Vec<RealmEdgeUserUpdateSubmission<F>>>;
    async fn peek_all(&self, checkpoint: UniqueQueueId) -> Result<Vec<RealmEdgeUserUpdateSubmission<F>>>;
    async fn delete_queue(&self, checkpoint: UniqueQueueId) -> Result<()>;
    fn get_queue_id(&self, checkpoint: &UniqueQueueId) -> QueueId;
    async fn get_queue_stats(&self, checkpoint: &UniqueQueueId) -> Result<QueueStats>;
    async fn has_messages(&self, checkpoint: &UniqueQueueId) -> Result<bool>;
    async fn get_message_count(&self, checkpoint: &UniqueQueueId) -> Result<u64>;
}

#[async_trait]
pub trait DuplicateTracker: Send + Sync {
    async fn has_submitted(&self, checkpoint: UniqueQueueId, user_id: u64) -> Result<bool>;
    async fn mark_submitted(&self, checkpoint: UniqueQueueId, user_id: u64) -> Result<()>;
    async fn clear_checkpoint(&self, checkpoint: UniqueQueueId) -> Result<()>;
}
