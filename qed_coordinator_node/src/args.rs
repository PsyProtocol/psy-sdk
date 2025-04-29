use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct CoordinatorWorkerArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub coordinator_redis_uri: String,
    #[clap(long, short, default_value = "8")]
    pub coordinator_pool_size: u32,
    #[clap(flatten)]
    pub coordinator_processor_queue_args: CoordinatorProcessorQueueArgs,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorProcessorArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub coordinator_redis_uri: String,
    #[clap(long, short, default_value = "8")]
    pub coordinator_pool_size: u32,
    #[clap(env, long, default_value = "./db/coordinator", env)]
    pub coordinator_db_path: String,
    #[clap(flatten)]
    pub coordinator_processor_queue_args: CoordinatorProcessorQueueArgs,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorProcessorQueueArgs {
    #[clap(long, short, default_value = "wq1")]
    pub coordinator_worker_queue_suffix: String,
    #[clap(long, short, default_value = "nq1")]
    pub coordinator_notifications_queue_suffix: String,
    #[clap(long, short, default_value = "CW")]
    pub coordinator_proof_store_key_suffix: String,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorEdgeArgs {
    #[clap(env, long, default_value = "redis://localhost:6379")]
    pub coordinator_redis_uri: String,
    #[clap(env, long, default_value = "0.0.0.0:8545")]
    pub coordinator_edge_listen_addr: String,
    #[clap(env, long, default_value = "./db/coordinator")]
    pub coordinator_db_path: String,
    #[clap(flatten)]
    pub coordinator_edge_queue_args: CoordinatorEdgeQueueArgs,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorEdgeQueueArgs {
    #[clap(long, short, default_value = "wq1")]
    pub coordinator_worker_queue_suffix: String,
    #[clap(long, short, default_value = "nq1")]
    pub coordinator_notifications_queue_suffix: String,
    #[clap(long, short, default_value = "RP")]
    pub realm_proof_store_key_suffix: String,
    #[clap(long, short, default_value = "CW")]
    pub coordinator_proof_store_key_suffix: String,
}