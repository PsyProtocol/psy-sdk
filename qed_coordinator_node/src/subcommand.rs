use clap::command;
use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // Edge(CoordinatorEdgeArgs),
    Processor(CoordinatorProcessorArgs),
    Worker(CoordinatorWorkerArgs),
}

#[derive(Clone, Args)]
pub struct CoordinatorWorkerArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub redis_url: String,
    #[clap(long, short, default_value = "8")]
    pub pool_size: u32,
}

#[derive(Clone, Args)]
pub struct CoordinatorProcessorArgs {
    #[clap(env, long, default_value = "redis://localhost:6379", env)]
    pub redis_uri: String,
    #[clap(long, short, default_value = "8")]
    pub pool_size: u32,
    #[clap(env, long, default_value = "./db/coordinator", env)]
    pub coordinator_db_path: String,
}
