use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct CoordinatorWorkerArgs {
    #[clap(
        env = "COORDINATOR_REDIS_URI",
        long,
        default_value = "redis://localhost:6379"
    )]
    pub redis_uri: String,
    #[clap(long, short, default_value = "20")]
    pub pool_size: u32,
    #[clap(flatten)]
    pub queue_args: CoordinatorQueueArgs,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorProcessorArgs {
    #[clap(
        env = "COORDINATOR_REDIS_URI",
        long,
        default_value = "redis://localhost:6379"
    )]
    pub redis_uri: String,
    #[clap(long, short, default_value = "20")]
    pub pool_size: u32,
    #[clap(env = "COORDINATOR_DB_PATH", long, default_value = "./db/coordinator")]
    pub db_path: String,
    #[clap(env = "COORDINATOR_DB_SIZE_GB", long, default_value = "100")]
    pub db_size_gb: usize,
    #[clap(flatten)]
    pub queue_args: CoordinatorQueueArgs,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorEdgeArgs {
    #[clap(
        env = "COORDINATOR_REDIS_URI",
        long,
        default_value = "redis://localhost:6379"
    )]
    pub redis_uri: String,
    #[clap(env = "COORDINATOR_LISTEN_ADDR", long, default_value = "0.0.0.0:8545")]
    pub listen_addr: String,
    #[clap(env = "COORDINATOR_DB_PATH", long, default_value = "./db/coordinator")]
    pub db_path: String,
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
        short,
        default_value = "CW"
    )]
    pub proof_store_key_suffix: String,
}
