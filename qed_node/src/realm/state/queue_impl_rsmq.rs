use std::marker::PhantomData;
use std::sync::Arc;
use async_trait::async_trait;
use plonky2::hash::hash_types::RichField;
use psy_store::queue::{QueueId, QueueStats, RsmqQueue};
use crate::common_v2::traits::realm::{RealmEdgeUserUpdateSubmission, UniqueQueueId};
use crate::realm::state::queue_traits::EdgeSubmissionQueue;
use anyhow::Result;
pub struct SubmissionQueue<F: RichField> {
    queue: Arc<RsmqQueue>,
    realm_id: u32,
    _phantom: PhantomData<F>,
}

impl<F: RichField> SubmissionQueue<F> {
    pub fn new(rsmq: Arc<RsmqQueue>, realm_id: u32) -> Self {
        Self {
            queue: rsmq,
            realm_id,
            _phantom: PhantomData,
        }
    }

    fn get_queue_id(&self, checkpoint: &UniqueQueueId) -> QueueId {
        QueueId::WorkerEvent {
            queue_biz_key: format!("realm_{}:uuid_{}",
                                   self.realm_id,
                                   checkpoint.uuid
            ),
        }
    }
}

#[async_trait]
impl<F: RichField + Send + Sync + 'static> EdgeSubmissionQueue<F> for SubmissionQueue<F> {
    async fn enqueue(&self, checkpoint: UniqueQueueId, submission: RealmEdgeUserUpdateSubmission<F>) -> Result<()> {
        let queue_id = self.get_queue_id(&checkpoint);
        self.queue.create_queue_if_not_exists(&queue_id).await?;

        let serialized = bincode::serialize(&submission)?;
        self.queue.send_message(&queue_id, serialized).await?;
        Ok(())
    }

    async fn enqueue_batch(&self, checkpoint: UniqueQueueId, submissions: Vec<RealmEdgeUserUpdateSubmission<F>>) -> Result<()> {
        if submissions.is_empty() {
            return Ok(());
        }

        let queue_id = self.get_queue_id(&checkpoint);
        self.queue.create_queue_if_not_exists(&queue_id).await?;

        let serialized_items: Vec<Vec<u8>> = submissions
            .into_iter()
            .map(|s| bincode::serialize(&s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;

        //todo! optimize batch enqueue if supported by RSMQ
        for serialized in serialized_items {
            self.queue.send_message(&queue_id, serialized).await?;
        }

        Ok(())
    }

    async fn drain_all(&self, checkpoint: UniqueQueueId) -> Result<Vec<RealmEdgeUserUpdateSubmission<F>>> {
        let queue_id = self.get_queue_id(&checkpoint);
        let submissions = self.queue.pop_all::<RealmEdgeUserUpdateSubmission<F>>(&queue_id).await?;
        let _ = self.queue.delete_queue(&queue_id).await;

        Ok(submissions)
    }

    async fn peek_all(&self, checkpoint: UniqueQueueId) -> Result<Vec<RealmEdgeUserUpdateSubmission<F>>> {
        let queue_id = self.get_queue_id(&checkpoint);
        let mut submissions = Vec::new();
        let mut msg_ids = Vec::new();

        // Receive messages with short visibility timeout
        while let Some((msg, id)) = self.queue
            .receive_object_with_id::<RealmEdgeUserUpdateSubmission<F>>(
                &queue_id,
                Some(std::time::Duration::from_secs(1))
            ).await?
        {
            submissions.push(msg);
            msg_ids.push(id);
        }

        // Immediately reset visibility to make messages available again
        for msg_id in msg_ids {
            let _ = self.queue.change_message_visibility(
                &queue_id,
                &msg_id,
                std::time::Duration::from_secs(0)
            ).await;
        }

        Ok(submissions)
    }

    async fn delete_queue(&self, checkpoint: UniqueQueueId) -> Result<()> {
        let queue_id = self.get_queue_id(&checkpoint);
        self.queue.delete_queue(&queue_id).await?;
        Ok(())
    }

    fn get_queue_id(&self, checkpoint: &UniqueQueueId) -> QueueId {
        self.get_queue_id(checkpoint)
    }

    async fn get_queue_stats(&self, checkpoint: &UniqueQueueId) -> Result<QueueStats> {
        let queue_id = self.get_queue_id(checkpoint);
        let stats = self.queue.get_queue_stats(&queue_id).await?;
        Ok(stats)

    }

    async fn has_messages(&self, checkpoint: &UniqueQueueId) -> Result<bool> {
        let stats = self.get_queue_stats(checkpoint).await?;
        Ok(stats.total_messages > 0)
    }

    async fn get_message_count(&self, checkpoint: &UniqueQueueId) -> Result<u64> {
        let stats = self.get_queue_stats(checkpoint).await?;
        Ok(stats.total_messages)

    }
}
