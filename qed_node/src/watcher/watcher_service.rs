use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::{DateTime, Utc};
use qed_core::{config::network_constants::SLOT_SIZE, job::id::QProvingJobDataID};
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::task_queue::QProvingTaskStoreImpl;
use qed_store::queue::{new_redis_async_pool, QueueId, RsmqQueue};
use qed_store::store::QEDStore;
use redis::AsyncCommands;
use rsmq::RsmqMessage;
use serde::{Deserialize, Serialize};
use tokio::sync::{Semaphore, RwLock};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};
use qed_data::config::store_config::QEDFelt;
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use crate::watcher::{api_client::ApiClient, config::WatcherConfig, events::WatcherMessage, schedule_tasks::{ExecutionTrigger, ScheduledTask, ScheduledTaskManager, TaskType}, watcher::{NodeInfo, WatcherSourceNodeType, TimeoutWatcher}, ApiClientConfig};
use crate::watcher::block_sync::BlockSyncService;
use crate::watcher::checkpoint_sender::CheckpointSenderService;
use crate::watcher::message_processor::MessageProcessor;
use crate::watcher::utils::get_queue_name;

const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_ATTEMPT_TTL: u64 = 3600;
const TASK_MONITOR_INTERVAL: u64 = 5;
const BLOCK_SYNC_TIMEOUT: u64 = 10;
const FAILURE_BACKOFF_THRESHOLD: u32 = 3;
const FAILURE_BACKOFF_DURATION: u64 = 30;
const BLOCK_METADATA_WAIT_BLOCK_NUM: u64 = 3;
const BLOCK_METADATA_WAIT_DURATION: u64 = BLOCK_METADATA_WAIT_BLOCK_NUM * SLOT_SIZE;

const CHECKPOINT_LEAF_FETCH_RETRY_COUNT: u32 = 3;
const CHECKPOINT_LEAF_FETCH_RETRY_DELAY: u64 = 5;

pub struct WatcherService {
    message_processor: MessageProcessor,
    block_height_sync_service: BlockSyncService,
    checkpoint_sender: Option<CheckpointSenderService>,
    timeout_watcher: Arc<TimeoutWatcher>,
}

impl WatcherService {
    pub async fn new(config: WatcherConfig) -> Result<Self> {
        info!("Initializing watcher service for node: {}", config.node_id);

        let node_info = Arc::new(NodeInfo {
            node_id: config.node_id.clone(),
            node_type: config.node_type,
        });

        let qed_store = Arc::new(
            QEDStore::from_backend(config.backend.to_backend())
            .await
            .map_err(|e| anyhow!("Database init failed: {}", e))?,
        );

        let redis_pool = Arc::new(
            new_redis_async_pool(&config.redis_uri, config.redis_pool_size)
            .await
            .map_err(|e| anyhow!("Redis pool init failed: {}", e))?,
        );

        let queue_name = get_queue_name(&config.queue_id.queue_biz_key);

        let rsmq_queue = Arc::new(
            RsmqQueue::new(&config.redis_uri, config.redis_pool_size, &queue_name)
                .await
                .map_err(|e| anyhow!("Failed to create RSMQ queue: {}", e))?,
        );


        let queue_id = QueueId::WatcherEvent {
            queue_biz_key: queue_name.clone(),
        };
        rsmq_queue.create_queue_if_not_exists(&queue_id).await?;

        let api_client = Arc::new(Self::build_api_client(&config)?);
        let shared_block_height = Arc::new(AtomicU64::new(0));

        // Fetch initial height
        let block_sync_service = BlockSyncService::new(
            qed_store.clone(),
            Arc::clone(&shared_block_height),
            config.node_type,
            config.block_sync_interval,
        );

        if let Ok(height) = block_sync_service.fetch_initial_height().await {
            info!("Block height initialized to {}", height);
        } else if let Err(e) = block_sync_service.fetch_initial_height().await {
            warn!("⚠️ Failed to fetch initial height: {}. Starting at 0", e);
        }


        let timeout_watcher = Arc::new(TimeoutWatcher::new(
            redis_pool.clone(),
            config.redis_uri.clone(),
            rsmq_queue.clone(),
            node_info.clone(),
            &queue_name,
        ));

        // Initialize message processor
        let message_processor = MessageProcessor::new(
            rsmq_queue,
            queue_id,
            api_client.clone(),
            redis_pool,
            config.node_id.clone(),
        );

        // Only create checkpoint sender for Coordinator nodes
        let checkpoint_sender = if config.node_type == WatcherSourceNodeType::Coordinator {
            info!("Initializing checkpoint sender (Coordinator node)");
            Some(CheckpointSenderService::new(
                qed_store.clone(),
                api_client,
                Arc::clone(&shared_block_height),
            ))
        } else {
            info!("Skipping checkpoint sender (Realm node)");
            None
        };

        Ok(Self {
            message_processor,
            block_height_sync_service: block_sync_service,
            checkpoint_sender,
            timeout_watcher,
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        info!("Starting Watcher Service");

        let checkpoint_future = async {
            match &self.checkpoint_sender {
                Some(sender) => sender.run().await,
                None => {
                    // For Realm nodes, create a future that never completes
                    std::future::pending::<Result<()>>().await
                }
            }
        };

        tokio::select! {
            result = self.message_processor.run() => {
                error!("Message processor stopped: {:?}", result);
                result.map_err(Into::into)
            }
            result = self.block_height_sync_service.run() => {
                error!("Block sync stopped: {:?}", result);
                result.map_err(Into::into)
            }
            result = self.timeout_watcher.start_monitoring() => {
                error!("Timeout monitor stopped: {:?}", result);
                result.map_err(Into::into)
            }
            result = checkpoint_future => {
                error!("Send checkpoint  stopped: {:?}", result);
                result.map_err(Into::into)
            }
        }
    }


    fn build_api_client(config: &WatcherConfig) -> anyhow::Result<ApiClient> {
        let realm_id = if config.node_type == WatcherSourceNodeType::Realm {
            Some(config.node_id.parse()?)
        } else {
            None
        };

        let mut api_config = ApiClientConfig::new(
            config.api_endpoint.clone(),
            config.node_id.clone(),
            config.node_type,
            realm_id,
        );

        if let Some(jwt_secret) = config.jwt_secret.clone() {
            info!("Configuring API client with JWT authentication");
            api_config = api_config.with_jwt_secret(jwt_secret);
        } else {
            warn!("⚠️ API client initialized without JWT. Telemetry may fail.");
        }

        ApiClient::with_config(api_config)
    }
}
