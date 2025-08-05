use crate::queue::fred_queue::{
    PS_DRAIN_QUEUE_KEY_PREFIX, PS_HISTORY_QUEUE_KEY_PREFIX, PS_NOTIFICATIONS_QUEUE_KEY_PREFIX,
    PS_WORKER_QUEUE_KEY_PREFIX, SyncProofQueue,
};
use async_trait::async_trait;
use qed_core::config::network_constants::{COORDINATOR_TO_REALM_CHANNEL, REALM_TO_COORDINATOR_CHANNEL};
use kvq::traits::KVQSerializable;
use qed_core::job::drain_queue::{
    CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable,
};
use qed_core::job::history_queue::{
    CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm, HQSerializable,
    HistoryQueueMetadata, HistoryQueueMetadataTagged,
};
use qed_core::job::id::{QProvingJobDataID, ProvingJobDataId, QWorkerJobBenchmark};
use qed_core::job::worker_queue::{
    WorkerEventReceiverAsyncImm, WorkerEventTransmitterAsyncImm, 
    WorkerEventReceiverSync, WorkerEventTransmitterSync
};
use rsmq::{PoolOptions, PooledRsmq, RedisBytes, RsmqConnection, RsmqError, RsmqOptions,
    RsmqConnectionSync, RsmqSync, RsmqMessage};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use tokio::time::sleep;
use tracing::{error, info};

pub enum QueueId {
    WorkerEvent {
        worker_queue_suffix: String,
    },
    WorkerNotification {
        notifications_queue_suffix: String,
    },
    CheckpointDrain {
        worker_queue_suffix: String,
        channel_id: u64,
        checkpoint_id: u64,
    },
    CheckpointHistory {
        channel_id: u64,
    },
    SyncProof {
        worker_queue_suffix: String,
    },
}

impl QueueId {
    pub fn get_queue_id(&self) -> String {
        match self {
            QueueId::WorkerEvent {
                worker_queue_suffix,
            } => format!("{}-{}", PS_WORKER_QUEUE_KEY_PREFIX, worker_queue_suffix),
            QueueId::WorkerNotification {
                notifications_queue_suffix,
            } => format!(
                "{}-{}",
                PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, notifications_queue_suffix
            ),
            QueueId::CheckpointDrain {
                worker_queue_suffix,
                channel_id,
                checkpoint_id,
            } => {
                let worker_queue_id =
                    format!("{}-{}", PS_WORKER_QUEUE_KEY_PREFIX, worker_queue_suffix);
                let checkpoint_queue_prefix =
                    format!("{}-{}", worker_queue_id, PS_DRAIN_QUEUE_KEY_PREFIX);
                format!(
                    "{}-{}_{}",
                    checkpoint_queue_prefix, channel_id, checkpoint_id
                )
            }
            QueueId::CheckpointHistory { channel_id } => {
                format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id)
            }
            QueueId::SyncProof {
                worker_queue_suffix,
            } => {
                format!("{}-REALM_PROOF", worker_queue_suffix)
            }
        }
    }
}

pub struct RsmqQueue {
    pub pool: PooledRsmq,
    pub worker_queue_suffix: String,
    pub notifications_queue_suffix: String,
}

impl Debug for RsmqQueue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RsmqQueue")
            .field("worker_queue_suffix", &self.worker_queue_suffix)
            .field(
                "notifications_queue_suffix",
                &self.notifications_queue_suffix,
            )
            .finish()
    }
}

pub async fn new_rsmq_pool(redis_url: &str, pool_size: usize) -> anyhow::Result<PooledRsmq> {
    let url = url::Url::parse(redis_url)?;
    let mut rsmq_option = RsmqOptions::default();
    if let Some(host) = url.host() {
        rsmq_option.host = host.to_string();
    }
    if let Some(port) = url.port() {
        rsmq_option.port = port;
    }
    let path = url.path();
    if path.starts_with('/') && path.len() > 1 {
        let db_index_str = &path[1..];
        let db = u8::from_str(db_index_str)?;
        rsmq_option.db = db;
    }
    info!(?redis_url, host=?rsmq_option.host, port=?rsmq_option.port, db=?rsmq_option.db, "new rsmq pool");
    let pool_option = PoolOptions {
        max_size: Some(pool_size as u32),
        min_idle: None,
    };
    let pool = PooledRsmq::new(rsmq_option, pool_option).await?;
    Ok(pool)
}

impl RsmqQueue {
    pub async fn new(
        redis_url: &str,
        pool_size: usize,
        worker_queue_suffix: impl ToString,
        notifications_queue_suffix: impl ToString,
    ) -> anyhow::Result<Self> {
        let pool = new_rsmq_pool(redis_url, pool_size).await?;
        let client = Self {
            pool,
            worker_queue_suffix: worker_queue_suffix.to_string(),
            notifications_queue_suffix: notifications_queue_suffix.to_string(),
        };
        Ok(client)
    }
    pub async fn create_queue_if_not_exists(&self, queue: &QueueId) -> anyhow::Result<()> {
        let queue_id = queue.get_queue_id();
        match self.pool.get_queue_attributes(&queue_id).await {
            Ok(_) => Ok(()),
            Err(RsmqError::QueueNotFound) => {
                let ret = self.pool.create_queue(&queue_id, None, None, None).await;
                match ret {
                    Ok(()) | Err(RsmqError::QueueExists) => Ok(()),
                    Err(err) => Err(err)?,
                }
            }
            Err(err) => Err(err)?,
        }
    }

    pub async fn send_message<E: Into<RedisBytes> + Send>(
        &self,
        queue: &QueueId,
        message: E,
    ) -> anyhow::Result<()> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        let bytes = message.into();
        self.pool.send_message(&queue_id, bytes, None).await?;
        Ok(())
    }
    pub async fn receive_message(
        &self,
        queue: &QueueId,
        hidden: Option<Duration>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        let message = self
            .pool
            .receive_message::<Vec<u8>>(&queue_id, hidden)
            .await?;
        Ok(message.map(|msg| msg.message))
    }

    pub async fn delete_message(&self, queue: &QueueId, message_id: &str) -> anyhow::Result<()> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        self.pool.delete_message(&queue_id, message_id).await?;
        Ok(())
    }

    pub async fn pop_message(&self, queue: &QueueId) -> anyhow::Result<Option<Vec<u8>>> {
        self.create_queue_if_not_exists(queue).await?;
        let queue_id = queue.get_queue_id();
        let message = self.pool.pop_message::<Vec<u8>>(&queue_id).await?;
        Ok(message.map(|msg| msg.message))
    }
}

#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for RsmqQueue {
    async fn cdq_push_imm<T: DQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let metadata: qed_core::job::drain_queue::DrainQueueMetadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        let queue_id = QueueId::CheckpointDrain {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
            channel_id: metadata.channel_id,
            checkpoint_id: metadata.checkpoint_id,
        };
        self.send_message(&queue_id, bytes).await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for RsmqQueue {
    async fn cdq_drain_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let queue_id = QueueId::CheckpointDrain {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
            channel_id,
            checkpoint_id,
        };
        let mut members = vec![];
        while let Some(message) = self.pop_message(&queue_id).await? {
            members.push(message);
        }
        members.into_iter().map(|x| T::from_bytes(&x)).collect()
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
            channel_id: metadata.channel_id,
        };
        self.send_message(&queue_id, bytes).await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointHistoryQueueConsumerAsyncImm for RsmqQueue {
    async fn chq_listen_from_imm<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let queue_id = QueueId::CheckpointHistory { channel_id };
        let mut results = vec![];
        let mut current_checkpoint_id = None;
        while let Some(bytes) = self.pop_message(&queue_id).await? {
            let checkpoint_id = u64::from_le_bytes(bytes[..8].try_into()?);
            if let Some(cur_id) = current_checkpoint_id {
                if cur_id + 1 != checkpoint_id {
                    anyhow::bail!(
                        "Get wrong checkpoint id, expect {}, but get {}",
                        cur_id,
                        checkpoint_id
                    );
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

    async fn wait_for_next_item_imm<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<T> {
        let queue_id = QueueId::CheckpointHistory { channel_id };
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
    
    async fn is_empty(&self) -> anyhow::Result<bool> {
        // Check if sync proof queue is empty
        let queue_id = QueueId::SyncProof {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
        };
        match self.pool.get_queue_attributes(&queue_id.get_queue_id()).await {
            Ok(attrs) => Ok(attrs.msgs == 0),
            Err(RsmqError::QueueNotFound) => Ok(true),
            Err(e) => Err(e.into()),
        }
    }
}

#[async_trait]
impl WorkerEventReceiverAsyncImm for RsmqQueue {
    async fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID> {
        let queue_id = QueueId::WorkerEvent {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
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
            worker_queue_suffix: self.worker_queue_suffix.clone(),
        };
        for job in jobs {
            let bytes = job.to_fixed_bytes().to_vec();
            self.send_message(&queue_id, bytes).await?;
        }
        Ok(())
    }

    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        let queue_id = QueueId::WorkerNotification {
            notifications_queue_suffix: self.notifications_queue_suffix.clone(),
        };
        let bytes = job.to_fixed_bytes().to_vec();
        self.send_message(&queue_id, bytes).await?;
        Ok(())
    }
}

#[async_trait]
impl WorkerEventTransmitterAsyncImm for RsmqQueue {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let queue_id = QueueId::WorkerEvent {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
        };
        for job in jobs {
            let bytes = job.to_fixed_bytes().to_vec();
            self.send_message(&queue_id, bytes).await?;
        }
        Ok(())
    }

    async fn wait_for_block_proving_jobs_imm(
        &self,
        _checkpoint_id: u64,
    ) -> anyhow::Result<QProvingJobDataID> {
        let queue_id = QueueId::WorkerNotification {
            notifications_queue_suffix: self.notifications_queue_suffix.clone(),
        };
        loop {
            if let Some(bytes) = self.pop_message(&queue_id).await? {
                if bytes.len() == 24 {
                    match QProvingJobDataID::try_from_byte_vec(&bytes) {
                        Ok(job) => {
                            if job.is_notify_complete() {
                                return Ok(job);
                            }
                        }
                        Err(err) => error!(
                            "error deserializing job id in wait_for_block_proving_jobs_imm: {:?}",
                            err
                        ),
                    }
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
    }
}

#[async_trait]
impl SyncProofQueue for RsmqQueue {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()> {
        let queue_id = QueueId::SyncProof {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
        };
        self.send_message(&queue_id, item.to_bytes()?).await
    }

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId> {
        let queue_id = QueueId::SyncProof {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
        };
        match self.pop_message(&queue_id).await? {
            None => {
                anyhow::bail!("No message in queue");
            }
            Some(bytes) => match ProvingJobDataId::from_bytes(&bytes) {
                Ok(id) => Ok(id),
                Err(err) => Err(anyhow::anyhow!(
                    "Failed to parse ProvingJobDataId: {:?}",
                    err
                )),
            },
        }
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

// Implement HistoryQueueMetadataTagged for CEQueueNotification
impl HistoryQueueMetadataTagged for CEQueueNotification {
    fn get_hq_metadata(&self) -> HistoryQueueMetadata {
        match self {
            CEQueueNotification::StartProduceBlock { next_checkpoint } => {
                HistoryQueueMetadata {
                    channel_id: COORDINATOR_TO_REALM_CHANNEL,
                    checkpoint_id: *next_checkpoint,
                    item_id: *next_checkpoint, // Using checkpoint as item_id
                }
            }
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
                if matches!(
                    queue.get_queue_attributes(*q),
                    Err(RsmqError::QueueNotFound)
                ) {
                    let _ = queue.create_queue(*q, Q_HIDDEN, Q_DELAY, Q_CAP);
                }
            }
            Ok::<_, anyhow::Error>(queue)
        }?;
        Ok(Self { queue })
    }

    pub fn ensure_queue(&mut self, name: &str) -> anyhow::Result<()> {
        if matches!(
            self.queue.get_queue_attributes(name),
            Err(rsmq::RsmqError::QueueNotFound)
        ) {
            // use Q_HIDDEN / Q_DELAY / Q_CAP
            self.queue.create_queue(name, Q_HIDDEN, Q_DELAY, Q_CAP)?;
            tracing::info!("🔧 RSMQ queue `{name}` created");
        }
        Ok(())
    }
}


// Wrapper types from wq_mut.rs
#[derive(Clone)]
pub struct QEDArcImmutableEventProcessorWrapper<P> {
    pub inner: Arc<RwLock<P>>,
}

impl<P> QEDArcImmutableEventProcessorWrapper<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}

#[derive(Clone)]
pub struct QEDRedisEventProcessor {
    pub job_queue: RedisQueue,
    pub benckmarks_enabled: bool,
    pub benchmarks: Vec<QWorkerJobBenchmark>,
}

impl QEDRedisEventProcessor {
    pub fn new(dispatcher: RedisQueue) -> Self {
        Self::new_with_config(dispatcher, false)
    }
    
    pub fn new_with_config(dispatcher: RedisQueue, benckmarks_enabled: bool) -> Self {
        Self {
            job_queue: dispatcher,
            benckmarks_enabled,
            benchmarks: Vec::new(),
        }
    }
    
    pub fn to_imm(self) -> QEDArcImmutableEventProcessorWrapper<Self> {
        QEDArcImmutableEventProcessorWrapper::new(self)
    }
}

impl WorkerEventReceiverSync for QEDRedisEventProcessor {
    fn wait_for_next_job_mut(&mut self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            match self.job_queue.queue.pop_message::<Vec<u8>>(Q_JOB)? {
                Some(RsmqMessage { message, .. }) => {
                    return Ok(serde_json::from_slice(&message)?)
                }
                None => {
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
            }
        }
    }

    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        for job in jobs {
            self.job_queue.queue.send_message(Q_JOB, serde_json::to_vec(&job)?, None)?;
        }
        Ok(())
    }

    fn notify_core_goal_completed_mut(&mut self, _job: QProvingJobDataID) -> anyhow::Result<()> {
        self.job_queue.queue.send_message(Q_NOTIFICATIONS, serde_json::to_vec(&QueueNotification::CoreJobCompleted)?, None)?;
        Ok(())
    }

    fn record_job_bench_mut(&mut self, job: QProvingJobDataID, duration: u64) -> anyhow::Result<()> {
        if self.benckmarks_enabled {
            self.benchmarks.push(QWorkerJobBenchmark {
                job_id: job.to_fixed_bytes(),
                duration,
            });
        }
        Ok(())
    }
}

impl WorkerEventTransmitterSync for QEDRedisEventProcessor {
    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        for job in jobs {
            self.job_queue.queue.send_message(Q_JOB, serde_json::to_vec(&job)?, None)?;
        }
        Ok(())
    }

    fn wait_for_block_proving_jobs_mut(&mut self, _checkpoint_id: u64) -> anyhow::Result<bool> {
        loop {
            match self.job_queue.queue.pop_message::<Vec<u8>>(Q_NOTIFICATIONS)? {
                Some(RsmqMessage { message, .. }) => match serde_json::from_slice::<QueueNotification>(&message) {
                    Ok(QueueNotification::CoreJobCompleted) => return Ok(true),
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                },
                None => {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }
    }
}
