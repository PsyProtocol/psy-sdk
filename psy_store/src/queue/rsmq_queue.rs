use std::{
    collections::HashSet,
    fmt::{Debug, Formatter},
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kvq::traits::KVQSerializable;
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use psy_config::network_constants::COORDINATOR_EDGE_TO_PROCESSOR_CHANNEL;
use psy_common::job::{
    drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable},
    history_queue::{
        CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm, HQSerializable, HistoryQueueMetadata,
        HistoryQueueMetadataTagged,
    },
    id::{QProvingJobDataID, QWorkerJobBenchmark},
    traits::QProofStoreReaderAsync,
    worker_queue::{WorkerEventReceiverAsyncImm, WorkerEventTransmitterAsyncImm},
};
use rsmq::{PoolOptions, PooledRsmq, RedisBytes, RsmqConnection, RsmqConnectionSync, RsmqError, RsmqMessage, RsmqOptions, RsmqSync};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::time::{interval, sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::queue::{
    BizKey, QueuePrefixKey, PS_DRAIN_QUEUE_KEY_PREFIX, PS_HISTORY_QUEUE_KEY_PREFIX, PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, PS_WORKER_QUEUE_KEY_PREFIX,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueId {
    WorkerEvent {
        queue_biz_key: String,
    },
    WorkerNotification {
        notifications_queue_suffix: String,
    },
    CheckpointDrain {
        queue_biz_key: String,
        channel_id: u64,
        checkpoint_id: u64,
    },
    CheckpointHistory {
        checkpoint_history_queue_prefix_key: String,
        channel_id: u64,
    },
    SyncProof {
        queue_biz_key: String,
    },
    WatcherEvent {
        queue_biz_key: String,
    },
}

impl QueueId {
    pub fn get_queue_id(&self) -> String {
        match self {
            QueueId::WorkerEvent { queue_biz_key } => queue_biz_key.clone(),
            QueueId::WorkerNotification { notifications_queue_suffix } => notifications_queue_suffix.clone(),
            QueueId::CheckpointDrain {
                queue_biz_key, channel_id, ..
            } => {
                format!("{}-{}", queue_biz_key, channel_id)
            }
            QueueId::CheckpointHistory {
                checkpoint_history_queue_prefix_key,
                channel_id,
            } => {
                format!("{}-{}", checkpoint_history_queue_prefix_key, channel_id)
            }
            QueueId::SyncProof { queue_biz_key } => queue_biz_key.clone(),
            QueueId::WatcherEvent { queue_biz_key } => queue_biz_key.clone(),
        }
    }
}
/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub queue_name: String,
    pub total_messages: u64,
    pub hidden_messages: u64,
    pub total_sent: u64,
    pub total_received: u64,
    pub created_at: u64,
    pub modified_at: u64,
}

// ===== RSMQ Pool Creation =====

/// Creates an RSMQ connection pool from Redis URL
pub async fn create_rsmq_pool(redis_url: &str, pool_size: usize) -> Result<PooledRsmq> {
    let url = url::Url::parse(redis_url)?;
    let mut rsmq_options = RsmqOptions::default();

    if let Some(host) = url.host() {
        rsmq_options.host = host.to_string();
    }

    if let Some(port) = url.port() {
        rsmq_options.port = port;
    }

    let path = url.path();
    if path.starts_with('/') && path.len() > 1 {
        let db_index_str = &path[1..];
        let db = u8::from_str(db_index_str)?;
        rsmq_options.db = db;
    }

    debug!(
        "Creating RSMQ pool - Host: {}, Port: {}, DB: {}, Pool size: {}",
        rsmq_options.host, rsmq_options.port, rsmq_options.db, pool_size
    );

    let pool_options = PoolOptions {
        max_size: Some(pool_size as u32),
        min_idle: Some((pool_size / 2) as u32),
    };

    Ok(PooledRsmq::new(rsmq_options, pool_options).await?)
}
// ===== Main Queue Implementation =====

/// Unified RSMQ queue implementation
pub struct RsmqQueue {
    pub pool: PooledRsmq,
    pub biz_key: String,
    tracked_queues: Arc<RwLock<HashSet<String>>>,
    cancellation_token: CancellationToken,
}

impl BizKey for RsmqQueue {
    fn biz_key(&self) -> String {
        self.biz_key.clone()
    }
}

impl Debug for RsmqQueue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RsmqQueue")
            .field("biz_key", &self.biz_key)
            .field("tracked_queues_count", &self.tracked_queues.read().map(|q| q.len()).unwrap_or(0))
            .finish()
    }
}

impl Drop for RsmqQueue {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}

impl RsmqQueue {
    pub async fn new(redis_url: &str, pool_size: usize, biz_key: impl ToString) -> Result<Self> {
        let pool = create_rsmq_pool(redis_url, pool_size).await?;
        let tracked_queues = Arc::new(RwLock::new(HashSet::new()));
        let cancellation_token = CancellationToken::new();

        // Start background cleanup task
        let pool_clone = pool.clone();
        let tracked_queues_clone = tracked_queues.clone();
        let cancellation_token_clone = cancellation_token.clone();

        tokio::spawn(async move {
            Self::cleanup_idle_queues(pool_clone, tracked_queues_clone, cancellation_token_clone).await;
        });

        Ok(Self {
            pool,
            biz_key: biz_key.to_string(),
            tracked_queues,
            cancellation_token,
        })
    }

    /// Core logic for cleaning up idle queues
    async fn cleanup_idle_queues(pool: PooledRsmq, tracked_queues: Arc<RwLock<HashSet<String>>>, cancellation_token: CancellationToken) {
        const CLEANUP_INTERVAL: Duration = Duration::from_secs(5 * 60); // 5 minutes
        const IDLE_THRESHOLD: u64 = 60 * 60; // 1 hour in seconds

        let mut interval = interval(CLEANUP_INTERVAL);
        interval.tick().await; // Skip the first immediate tick

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    info!("Cleanup task received cancellation signal, exiting");
                    break;
                }
                _ = interval.tick() => {
                    let queue_names = {
                        let queues = tracked_queues.read().unwrap();
                        queues.iter().cloned().collect::<Vec<_>>()
                    };

                    debug!("Starting idle queue cleanup, tracking {} queues", queue_names.len());

                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    for queue_name in queue_names {
                        match pool.get_queue_attributes(&queue_name).await {
                            Ok(attr) => {
                                // Check if queue is empty and hasn't been modified for over 1 hour
                                if attr.msgs == 0 && attr.hiddenmsgs == 0 {
                                    let idle_duration = current_time.saturating_sub(attr.modified);

                                    if idle_duration >= IDLE_THRESHOLD {
                                        debug!(
                                            "Queue {} is empty and idle for {} seconds, preparing to delete",
                                            queue_name, idle_duration
                                        );

                                        match pool.delete_queue(&queue_name).await {
                                            Ok(_) => {
                                                info!("🗑️ Deleted idle queue: {}", queue_name);
                                                // Remove from tracking list
                                                let mut queues = tracked_queues.write().unwrap();
                                                queues.remove(&queue_name);
                                            }
                                            Err(e) => {
                                                error!("Failed to delete queue {}: {:?}", queue_name, e);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(RsmqError::QueueNotFound) => {
                                // Queue no longer exists, remove from tracking list
                                debug!("Queue {} does not exist, removing from tracking list", queue_name);
                                let mut queues = tracked_queues.write().unwrap();
                                queues.remove(&queue_name);
                            }
                            Err(e) => {
                                error!("Failed to get queue {} attributes: {:?}", queue_name, e);
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn create_queue_if_not_exists(&self, queue: &QueueId) -> Result<()> {
        let queue_id = queue.get_queue_id();
        let ret = match self.pool.get_queue_attributes(&queue_id).await {
            Ok(_) => Ok(()),
            Err(RsmqError::QueueNotFound) => match self.pool.create_queue(&queue_id, None, None, None).await {
                Ok(()) | Err(RsmqError::QueueExists) => Ok(()),
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err.into()),
        };
        if ret.is_ok() {
            let mut queues = self.tracked_queues.write().unwrap();
            queues.insert(queue_id);
        }
        ret
    }

    pub async fn send_message<E: Into<RedisBytes> + Send>(&self, queue: &QueueId, message: E) -> Result<()> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        self.pool.send_message(&queue_id, message, None).await?;
        Ok(())
    }
    pub async fn receive_message(&self, queue: &QueueId, hidden: Option<Duration>) -> Result<Option<Vec<u8>>> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        let message = self.pool.receive_message::<Vec<u8>>(&queue_id, hidden).await?;
        Ok(message.map(|msg| msg.message))
    }

    pub async fn receive_message_with_id(&self, queue: &QueueId, hidden: Option<Duration>) -> Result<Option<RsmqMessage<Vec<u8>>>> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        Ok(self.pool.receive_message(&queue_id, hidden).await?)
    }

    pub async fn delete_message(&self, queue: &QueueId, message_id: &str) -> Result<()> {
        let queue_id = queue.get_queue_id();
        self.pool.delete_message(&queue_id, message_id).await?;
        Ok(())
    }

    pub async fn pop_message(&self, queue: &QueueId) -> Result<Option<Vec<u8>>> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        let message = self.pool.pop_message::<Vec<u8>>(&queue_id).await?;
        Ok(message.map(|msg| msg.message))
    }

    // ===== Type-safe serialization methods =====

    /// Send a serializable object to the queue
    pub async fn send_object<T: Serialize>(&self, queue: &QueueId, obj: &T) -> Result<()> {
        let bytes = bincode::serialize(obj)?;
        self.send_message(queue, bytes).await
    }

    /// Send a JSON-serializable object to the queue
    pub async fn send_json<T: Serialize>(&self, queue: &QueueId, obj: &T) -> Result<()> {
        let json = serde_json::to_vec(obj)?;
        self.send_message(queue, json).await
    }

    /// Receive and deserialize an object from the queue
    pub async fn receive_object<T: for<'de> Deserialize<'de>>(&self, queue: &QueueId, hidden: Option<Duration>) -> Result<Option<T>> {
        match self.receive_message(queue, hidden).await? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Receive and deserialize a JSON object from the queue
    pub async fn receive_json<T: for<'de> Deserialize<'de>>(&self, queue: &QueueId, hidden: Option<Duration>) -> Result<Option<T>> {
        match self.receive_message(queue, hidden).await? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Receive an object with message ID for acknowledgment
    pub async fn receive_object_with_id<T: for<'de> Deserialize<'de>>(
        &self,
        queue: &QueueId,
        hidden: Option<Duration>,
    ) -> Result<Option<(T, String)>> {
        match self.receive_message_with_id(queue, hidden).await? {
            Some(msg) => {
                let obj = bincode::deserialize(&msg.message)?;
                Ok(Some((obj, msg.id)))
            }
            None => Ok(None),
        }
    }

    /// Pop and deserialize an object from the queue
    pub async fn pop_object<T: for<'de> Deserialize<'de>>(&self, queue: &QueueId) -> Result<Option<T>> {
        match self.pop_message(queue).await? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Pop and deserialize a JSON object from the queue
    pub async fn pop_json<T: for<'de> Deserialize<'de>>(&self, queue: &QueueId) -> Result<Option<T>> {
        match self.pop_message(queue).await? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Send multiple objects in batch
    pub async fn send_batch<T: Serialize>(&self, queue: &QueueId, items: &[T]) -> Result<()> {
        for item in items {
            self.send_object(queue, item).await?;
        }
        Ok(())
    }

    /// Receive multiple objects (up to limit)
    pub async fn receive_batch<T: for<'de> Deserialize<'de>>(&self, queue: &QueueId, limit: usize, hidden: Option<Duration>) -> Result<Vec<T>> {
        let mut results = Vec::with_capacity(limit.min(100));
        for _ in 0..limit {
            match self.receive_object(queue, hidden).await? {
                Some(obj) => results.push(obj),
                None => break,
            }
        }
        Ok(results)
    }

    /// Pop all messages from queue and deserialize
    pub async fn pop_all<T: for<'de> Deserialize<'de>>(&self, queue: &QueueId) -> Result<Vec<T>> {
        let mut results = Vec::new();
        while let Some(obj) = self.pop_object(queue).await? {
            results.push(obj);
        }
        Ok(results)
    }

    pub async fn get_queue_length(&self, queue: &QueueId) -> Result<u64> {
        let queue_id = queue.get_queue_id();
        match self.pool.get_queue_attributes(&queue_id).await {
            Ok(attr) => Ok(attr.msgs),
            Err(RsmqError::QueueNotFound) => Ok(0),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn delete_queue(&self, queue: &QueueId) -> Result<()> {
        let queue_id = queue.get_queue_id();
        self.pool.delete_queue(&queue_id).await?;
        Ok(())
    }

    pub async fn get_queue_stats(&self, queue: &QueueId) -> Result<QueueStats> {
        let queue_id = queue.get_queue_id();
        match self.pool.get_queue_attributes(&queue_id).await {
            Ok(attr) => Ok(QueueStats {
                queue_name: queue_id,
                total_messages: attr.msgs,
                hidden_messages: attr.hiddenmsgs,
                total_sent: attr.totalsent,
                total_received: attr.totalrecv,
                created_at: attr.created,
                modified_at: attr.modified,
            }),
            Err(RsmqError::QueueNotFound) => Ok(QueueStats {
                queue_name: queue_id,
                ..Default::default()
            }),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn change_message_visibility(&self, queue: &QueueId, message_id: &str, visibility: Duration) -> Result<()> {
        let queue_id = queue.get_queue_id();

        // RSMQ uses this Redis command internally
        self.pool.change_message_visibility(&queue_id, message_id, visibility).await?;

        Ok(())
    }
}

// ===== Trait Implementations =====

#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for RsmqQueue {
    async fn cdq_push_imm<T: DQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let metadata: psy_common::job::drain_queue::DrainQueueMetadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        let queue_id = QueueId::CheckpointDrain {
            queue_biz_key: self.worker_queue_key().clone(),
            channel_id: metadata.channel_id,
            checkpoint_id: metadata.checkpoint_id,
        };
        self.send_message(&queue_id, bytes).await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for RsmqQueue {
    async fn cdq_drain_imm<T: DQSerializable>(&self, channel_id: u64) -> anyhow::Result<Vec<T>> {
        let queue_id = QueueId::CheckpointDrain {
            queue_biz_key: self.worker_queue_key().clone(),
            channel_id,
            checkpoint_id: 0, //todo remove this field
        };
        let mut members = vec![];
        while let Some(message) = self.pop_message(&queue_id).await? {
            members.push(message);
        }
        members.into_iter().map(|x| T::from_bytes(&x)).collect()
    }

    async fn cdq_peek_imm<T: DQSerializable>(&self, channel_id: u64) -> anyhow::Result<Vec<T>> {
        let queue_id = QueueId::CheckpointDrain {
            queue_biz_key: self.worker_queue_key().clone(),
            channel_id,
            checkpoint_id: 0, // Not used for peeking
        };

        let mut members = vec![];
        let mut peeked_messages = vec![];

        // Receive messages with visibility timeout to peek at them
        // We'll collect them and then change visibility back to 0
        while let Some(msg) = self.receive_message_with_id(&queue_id, Some(Duration::from_secs(1))).await? {
            let data = T::from_bytes(&msg.message)?;
            members.push(data);
            peeked_messages.push(msg.id);

            // Limit peek to reasonable amount to avoid blocking
            if members.len() >= 100 {
                break;
            }
        }

        // Reset visibility for all peeked messages to make them immediately available
        // again
        for msg_id in peeked_messages {
            // Set visibility to 0 to make message immediately available
            if let Err(e) = self.change_message_visibility(&queue_id, &msg_id, Duration::from_secs(0)).await {
                // Log but don't fail - message might have been consumed by another process
                debug!("Failed to reset visibility for message {}: {:?}", msg_id, e);
            }
        }

        Ok(members)
    }

    async fn cdq_len_imm(&self, channel_id: u64) -> anyhow::Result<usize> {
        let queue_id = QueueId::CheckpointDrain {
            queue_biz_key: self.worker_queue_key().clone(),
            channel_id,
            checkpoint_id: 0, // Not used for length check
        };
        let count = self.get_queue_stats(&queue_id).await?.total_messages;
        Ok(count as usize)
    }
}

#[async_trait]
impl CheckpointHistoryQueueEmitterAsyncImm for RsmqQueue {
    async fn chq_push_imm<T: HQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let metadata = item.get_hq_metadata();
        let checkpoint_id = metadata.checkpoint_id;
        let mut bytes = vec![];
        bytes.extend(checkpoint_id.to_le_bytes().to_vec());
        bytes.extend(item.to_bytes()?);
        let queue_id = QueueId::CheckpointHistory {
            checkpoint_history_queue_prefix_key: self.checkpoint_history_queue_prefix_key().clone(),
            channel_id: metadata.channel_id,
        };
        self.send_message(&queue_id, bytes).await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointHistoryQueueConsumerAsyncImm for RsmqQueue {
    async fn chq_items_gte<T: HQSerializable>(&self, channel_id: u64, start_checkpoint_id: u64) -> anyhow::Result<Vec<T>> {
        let queue_id = QueueId::CheckpointHistory {
            checkpoint_history_queue_prefix_key: self.checkpoint_history_queue_prefix_key().clone(),
            channel_id,
        };
        let mut results = vec![];
        let mut current_checkpoint_id = None;
        while let Some(bytes) = self.pop_message(&queue_id).await? {
            let checkpoint_id = u64::from_le_bytes(bytes[..8].try_into()?);
            if let Some(cur_id) = current_checkpoint_id {
                if cur_id + 1 != checkpoint_id {
                    return Err(anyhow!("Wrong checkpoint id, expect {}, but got {}", cur_id + 1, checkpoint_id));
                }
            }
            current_checkpoint_id = Some(checkpoint_id);
            if checkpoint_id >= start_checkpoint_id {
                let result = &bytes[8..];
                results.push(T::from_bytes(&result)?);
            }
        }
        Ok(results)
    }

    async fn wait_for_next_item_imm<T: HQSerializable>(&self, channel_id: u64, start_checkpoint_id: u64) -> anyhow::Result<T> {
        let queue_id = QueueId::CheckpointHistory {
            checkpoint_history_queue_prefix_key: self.checkpoint_history_queue_prefix_key().clone(),
            channel_id,
        };
        let start_i64 = start_checkpoint_id;
        loop {
            sleep(Duration::from_millis(100)).await;
            let bytes = self.pop_message(&queue_id).await?;
            if let Some(bytes) = bytes {
                let checkpoint_id = u64::from_le_bytes(bytes[..8].try_into()?);
                if checkpoint_id >= start_i64 {
                    let result = &bytes[8..];
                    return Ok(T::from_bytes(&result)?);
                }
            };
        }
    }
}

#[async_trait]
impl WorkerEventReceiverAsyncImm for RsmqQueue {
    async fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID> {
        let queue_id = QueueId::WorkerEvent {
            queue_biz_key: self.worker_queue_key().clone(),
        };
        loop {
            if let Some(bytes) = self.pop_message(&queue_id).await? {
                let job_id = QProvingJobDataID::from_bytes(&bytes)?;
                return Ok(job_id);
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let queue_id = QueueId::WorkerEvent {
            queue_biz_key: self.worker_queue_key().clone(),
        };
        for job in jobs {
            let bytes = job.to_fixed_bytes().to_vec();
            self.send_message(&queue_id, bytes).await?;
        }
        Ok(())
    }

    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        let queue_id = QueueId::WorkerNotification {
            notifications_queue_suffix: self.notifications_queue_key().clone(),
        };
        let bytes = job.to_fixed_bytes().to_vec();
        self.send_message(&queue_id, bytes).await?;
        Ok(())
    }
}

// Sync queue types from worker_queue_redis
pub const Q_HIDDEN: Option<Duration> = Some(Duration::from_secs(600));
pub const Q_DELAY: Option<Duration> = None;
pub const Q_CAP: Option<i32> = Some(-1);

pub const Q_RPC_TOKEN_TRANSFER: &'static str = "RPC_TOKEN_TRANSFER";
pub const Q_RPC_CLAIM_DEPOSIT: &'static str = "RPC_CLAIM_DEPOSIT";
pub const Q_RPC_ADD_WITHDRAWAL: &'static str = "RPC_ADD_WITHDRAWAL";
pub const Q_RPC_REGISTER_USER: &'static str = "RPC_REGISTER_USER";

pub const Q_CMD: &'static str = "CMD";
pub const Q_JOB: &'static str = "JOB";
pub const Q_NOTIFICATIONS: &'static str = "NOTIFICATIONS";
pub const CE_NOTIFICATIONS: &'static str = "CE_NOTIFICATIONS";

#[derive(Clone, Copy, PartialEq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum QueueCmd {
    ProduceBlock = 0,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum QueueNotification {
    CoreJobCompleted = 0,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CEQueueNotification {
    StartProduceBlock { next_checkpoint: u64 },
}

// Channel IDs are imported from network_constants

// Implement KVQSerializable for CEQueueNotification
impl KVQSerializable for CEQueueNotification {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl HistoryQueueMetadataTagged for CEQueueNotification {
    fn get_hq_metadata(&self) -> HistoryQueueMetadata {
        match self {
            CEQueueNotification::StartProduceBlock { next_checkpoint } => HistoryQueueMetadata {
                channel_id: COORDINATOR_EDGE_TO_PROCESSOR_CHANNEL,
                checkpoint_id: *next_checkpoint,
                item_id: *next_checkpoint,
            },
        }
    }
}

#[derive(Clone)]
pub struct RedisQueue {
    // we use queue here because pubsub is mpmc
    queue: RsmqSync,
}

impl RedisQueue {
    pub fn new(uri: &str) -> anyhow::Result<Self> {
        let queue = {
            let url = url::Url::parse(uri)?;
            let mut rsmq_option = RsmqOptions::default();
            if let Some(host) = url.host() {
                rsmq_option.host = host.to_string();
            }
            if let Some(port) = url.port() {
                rsmq_option.port = port;
            }
            let mut queue = RsmqSync::new(rsmq_option)?;
            for q in &[
                Q_RPC_TOKEN_TRANSFER,
                Q_RPC_CLAIM_DEPOSIT,
                Q_RPC_ADD_WITHDRAWAL,
                Q_RPC_REGISTER_USER,
                Q_CMD,
                Q_JOB,
                Q_NOTIFICATIONS,
                CE_NOTIFICATIONS,
            ] {
                if matches!(queue.get_queue_attributes(*q), Err(RsmqError::QueueNotFound)) {
                    let _ = queue.create_queue(*q, Q_HIDDEN, Q_DELAY, Q_CAP);
                }
            }
            Ok::<_, anyhow::Error>(queue)
        }?;
        Ok(Self { queue })
    }

    pub fn ensure_queue(&mut self, name: &str) -> anyhow::Result<()> {
        if matches!(self.queue.get_queue_attributes(name), Err(rsmq::RsmqError::QueueNotFound)) {
            // use Q_HIDDEN / Q_DELAY / Q_CAP
            self.queue.create_queue(name, Q_HIDDEN, Q_DELAY, Q_CAP)?;
            tracing::info!("🔧 RSMQ queue `{name}` created");
        }
        Ok(())
    }
}
