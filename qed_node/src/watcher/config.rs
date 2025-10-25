use std::str::FromStr;
use qed_store::store::backend::BackendConfig;
use anyhow::Result;
use crate::watcher::timeout_watcher::WatcherSourceNodeType;
use clap::Args;
use qed_store::queue::QueueId;
use crate::realm::QueueConfig;

#[derive(Clone, Debug, Args)]
pub struct WatcherArgs {
    #[clap(
        long = "node-id",
        env = "WATCHER_NODE_ID",
        help = "Unique identifier for this watcher instance"
    )]
    pub node_id: String,

    #[clap(
        long = "node-type",
        env = "WATCHER_NODE_TYPE",
        help = "Type of node being monitored (coordinator, realm)",
        value_parser = ["coordinator", "realm"]
    )]
    pub node_type: String,

    #[clap(
        long = "api-endpoint",
        env = "WATCHER_API_ENDPOINT",
        help = "Data center API endpoint for reporting",
        default_value = "http://localhost:3000"
    )]
    pub api_endpoint: String,

    #[clap(
        long = "redis-uri",
        env = "WATCHER_REDIS_URL",
        help = "Redis connection URL",
        default_value = "redis://127.0.0.1:6379"
    )]
    pub redis_uri: String,

    #[clap(
        long = "redis-pool-size",
        env = "WATCHER_REDIS_POOL_SIZE",
        help = "Size of Redis connection pool",
        default_value = "20"
    )]
    pub redis_pool_size: usize,

    #[clap(
        long = "queue-biz-key",
        env = "WATCHER_QUEUE",
        help = "Name of the RSMQ queue",
        default_value = "watcher:queue"
    )]
    pub queue_biz_key: String,

    #[clap(
        long = "block-sync-interval",
        env = "WATCHER_BLOCK_SYNC_INTERVAL",
        help = "Interval in seconds for syncing block height",
        default_value = "10"
    )]
    pub block_sync_interval: u64,

    #[clap(
        long = "jwt-secret",
        env = "JWT_SECRET",
        help = "JWT secret for authenticating with telemetry endpoints (shared with API server)",
        required = false
    )]
    pub jwt_secret: Option<String>,

    #[clap(flatten)]
    pub backend: BackendConfig,
}

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub node_id: String,
    pub node_type: WatcherSourceNodeType,
    pub api_endpoint: String,
    pub redis_uri: String,
    pub redis_pool_size: usize,
    pub block_sync_interval: u64,
    pub backend: BackendConfig,
    pub queue_id: QueueConfig,
    pub jwt_secret: Option<String>,
}

impl WatcherConfig {
    /// Create config from command line arguments
    pub fn from_args(args: WatcherArgs) -> Result<Self> {
        let node_type = WatcherSourceNodeType::from_str(&args.node_type)?;
        let queue_id = QueueConfig::from_str(&args.queue_biz_key)?;

        // Try to load JWT secret from environment if not provided in args
        let jwt_secret = args.jwt_secret.or_else(|| {
            dotenv::dotenv().ok();
            std::env::var("JWT_SECRET").ok()
        });

        if jwt_secret.is_none() {
            tracing::warn!(
                "JWT_SECRET not configured. Telemetry endpoints may fail authentication. \
                Set JWT_SECRET environment variable or use --jwt-secret flag."
            );
        } else {
            tracing::info!("JWT authentication configured for telemetry endpoints");
        }


        Ok(Self {
            node_id: args.node_id,
            node_type,
            api_endpoint: args.api_endpoint,
            redis_uri: args.redis_uri,
            redis_pool_size: args.redis_pool_size,
            queue_id,
            block_sync_interval: args.block_sync_interval,
            backend: args.backend,
            jwt_secret,
        })
    }
}