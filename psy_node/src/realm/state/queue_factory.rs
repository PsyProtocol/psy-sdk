// In psy_node/src/realm/state/queue_factory.rs

use std::sync::Arc;

use anyhow::Result;
use plonky2::hash::hash_types::RichField;
use psy_store::{
    queue::{new_redis_async_pool, RsmqQueue},
    store::QEDStore,
};

use crate::realm::state::{duplicate_tracker::RedisDuplicateTracker, edge_queue_helper::RealmEdgeQueueHelper, queue_impl_rsmq::SubmissionQueue};

pub struct QueueFactory;

impl QueueFactory {
    pub async fn create_rsmq_helper<F: RichField + Send + Sync + 'static>(
        redis_url: &str,
        pool_size: usize,
        realm_id: u32,
        store: Arc<QEDStore>,
    ) -> Result<RealmEdgeQueueHelper<F>> {
        let rsmq = Arc::new(RsmqQueue::new(redis_url, pool_size, format!("realm_{}_queue", realm_id)).await?);

        let redis_pool = new_redis_async_pool(redis_url, pool_size).await?;

        let queue = Arc::new(SubmissionQueue::<F>::new(rsmq, realm_id));
        let tracker = Arc::new(RedisDuplicateTracker::new(redis_pool, realm_id));

        Ok(RealmEdgeQueueHelper::new(queue, tracker, redis_url, pool_size, store).await)
    }
}
