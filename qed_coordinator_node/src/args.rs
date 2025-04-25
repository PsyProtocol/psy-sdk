use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct CoordinatorWorkerArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub coordinator_redis_uri: String,
    #[clap(long, short, default_value = "8")]
    pub coordinator_pool_size: u32,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorProcessorArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub coordinator_redis_uri: String,
    #[clap(long, short, default_value = "8")]
    pub coordinator_pool_size: u32,
    #[clap(env, long, default_value = "./db/coordinator", env)]
    pub coordinator_db_path: String,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorEdgeArgs {
    #[clap(env, long, default_value = "redis://localhost:6379")]
    pub coordinator_redis_uri: String,
    #[clap(env, long, default_value = "0.0.0.0:8545")]
    pub coordinator_edge_listen_addr: String,
    #[clap(env, long, default_value = "./db/coordinator")]
    pub coordinator_db_path: String,
}
