use clap::command;
use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Clone, Debug, Args)]
pub struct CoordinatorWorkerArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub redis_url: String,
    #[clap(long, short, default_value = "8")]
    pub pool_size: u32,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorProcessorArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub redis_uri: String,
    #[clap(long, short, default_value = "8")]
    pub pool_size: u32,
    #[clap(env, long, default_value = "./db/coordinator", env)]
    pub coordinator_db_path: String,
}

#[derive(Clone, Debug, Args)]
pub struct CoordinatorEdgeArgs {
    #[clap(env, long, default_value = "redis://localhost:6379")]
    pub redis_url: String,
    #[clap(env, long, default_value = "8545")]
    pub coordinator_edge_port: u16,
    #[clap(env, long, default_value = "./db/coordinator")]
    pub coordinator_db_path: String,
}
