use clap::Args;
use psy_store::store::backend::BackendConfig;

#[derive(Clone, Debug, Args)]
pub struct CoordinatorWorkerArgs {
    #[clap(
        env = "COORDINATOR_REDIS_URI",
        long,
        default_value = "redis://127.0.0.1:6379"
    )]
    pub redis_uri: String,
    #[clap(long = "redis-pool-size", short = 'r', default_value_t = 20)]
    pub redis_pool_size: usize,
    #[clap(long = "edge-url", default_value = "http://localhost:8545")]
    pub edge_url: String,
    #[clap(flatten)]
    pub queue_args: CoordinatorQueueArgs,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorProcessorArgs {
    #[clap(
        env = "COORDINATOR_REDIS_URI",
        long,
        default_value = "redis://127.0.0.1:6379"
    )]
    pub redis_uri: String,
    #[clap(long = "redis-pool-size", short = 'r', default_value_t = 20)]
    pub redis_pool_size: usize,
    #[clap(flatten)]
    pub backend: BackendConfig,
    #[clap(flatten)]
    pub queue_args: CoordinatorQueueArgs,
    #[arg(long, help = "Path to configuration file", default_value = "config.json")]
    pub config_path: String,
    #[arg(long, env = "PROCESSED_CONTRACTS_MAX_SIZE", default_value = "64")]
    pub max_processed_contracts_per_block: Option<isize>,
    #[arg(long, env = "PROCESSED_USERS_MAX_SIZE", default_value = "256")]
    pub max_processed_users_per_block: Option<isize>,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorEdgeArgs {
    #[clap(
        env = "COORDINATOR_REDIS_URI",
        long,
        default_value = "redis://127.0.0.1:6379"
    )]
    pub redis_uri: String,
    #[clap(env = "COORDINATOR_LISTEN_ADDR", long, default_value = "0.0.0.0:8545")]
    pub listen_addr: String,
    #[clap(flatten)]
    pub backend: BackendConfig,
    #[clap(flatten)]
    pub queue_args: CoordinatorQueueArgs,
    #[clap(long = "redis-pool-size", short = 'r', default_value_t = 20)]
    pub redis_pool_size: usize,
    //worker white list file path
    #[arg(long, help = "Path to configuration file", default_value = "config.json")]
    pub config_path: String,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorQueueArgs {
    #[clap(
        env = "COORDINATOR_QUEUE_BIZ_KEY",
        long,
        short,
        default_value = "wq1"
    )]
    pub queue_biz_key: String,
}
