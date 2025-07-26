use clap::Args;
use qed_store::store::backend::BackendConfig;

#[derive(Clone, Debug, Args)]
pub struct CoordinatorWorkerArgs {
    #[clap(
        env = "COORDINATOR_REDIS_URI",
        long,
        default_value = "redis://127.0.0.1:6379"
    )]
    pub redis_uri: String,
    #[clap(long = "redis-pool-size", short = 'r', default_value_t = 20)]
    pub redis_pool_size: u32,
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
    pub redis_pool_size: u32,
    #[clap(flatten)]
    pub backend: BackendConfig,
    #[clap(flatten)]
    pub queue_args: CoordinatorQueueArgs,
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
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorQueueArgs {
    #[clap(
        env = "COORDINATOR_WORKER_QUEUE_SUFFIX",
        long,
        short,
        default_value = "wq1"
    )]
    pub worker_queue_suffix: String,
    #[clap(
        env = "COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX",
        long,
        short,
        default_value = "nq1"
    )]
    pub notifications_queue_suffix: String,
    #[clap(
        env = "COORDINATOR_PROOF_STORE_KEY_SUFFIX",
        long,
        short = 'k',
        default_value = "CW"
    )]
    pub proof_store_key_suffix: String,
}
