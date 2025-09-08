use std::str::FromStr;
use qed_store::store::backend::BackendConfig;
use anyhow::Result;
use crate::watcher::watcher::NodeType;
use clap::Args;

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
        help = "Type of node being monitored (coordinator, realm, worker)",
        value_parser = ["coordinator", "realm", "worker"]
    )]
    pub node_type: String,

    #[clap(
        long = "api-endpoint",
        env = "WATCHER_API_ENDPOINT",
        help = "Data center API endpoint for reporting",
        default_value = "http://localhost:8080"
    )]
    pub api_endpoint: String,

    #[clap(
        long = "redis-url",
        env = "WATCHER_REDIS_URL",
        help = "Redis connection URL",
        default_value = "redis://127.0.0.1:6379"
    )]
    pub redis_url: String,

    #[clap(
        long = "redis-pool-size",
        env = "WATCHER_REDIS_POOL_SIZE",
        help = "Size of Redis connection pool",
        default_value = "20"
    )]
    pub redis_pool_size: usize,

    #[clap(
        long = "queue-name",
        env = "WATCHER_QUEUE_NAME",
        help = "Name of the RSMQ queue",
        default_value = "watcher:queue"
    )]
    pub queue_name: String,

    #[clap(
        long = "block-sync-interval",
        env = "WATCHER_BLOCK_SYNC_INTERVAL",
        help = "Interval in seconds for syncing block height",
        default_value = "10"
    )]
    pub block_sync_interval: u64,

    #[clap(flatten)]
    pub backend: BackendConfig,
}

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub node_id: String,
    pub node_type: NodeType,
    pub api_endpoint: String,
    pub redis_url: String,
    pub redis_pool_size: usize,
    pub block_sync_interval: u64,
    pub backend: BackendConfig,
}

impl WatcherConfig {
    /// Create config from command line arguments
    pub fn from_args(args: WatcherArgs) -> Result<Self> {
        let node_type = NodeType::from_str(&args.node_type)?;

        Ok(Self {
            node_id: args.node_id,
            node_type,
            api_endpoint: args.api_endpoint,
            redis_url: args.redis_url,
            redis_pool_size: args.redis_pool_size,
            block_sync_interval: args.block_sync_interval,
            backend: args.backend,
        })
    }
}