use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod coordinator_edge;
pub mod coordinator_processor;
pub mod coordinator_worker;
pub mod realm_edge;
pub mod realm_processor;
pub mod realm_worker;

#[derive(Parser)]
pub struct Cli {
    #[arg(
        long = "log-level",
        default_value = "info",
        help = "Set the log level (error, warn, info, debug, trace)"
    )]
    pub log_level: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Run the coordinator edge node")]
    CoordinatorEdge(qed_node::coordinator::CoordinatorEdgeArgs),
    #[command(about = "Run the coordinator processor node")]
    CoordinatorProcessor(qed_node::coordinator::CoordinatorProcessorArgs),
    #[command(about = "Run the coordinator worker node")]
    CoordinatorWorker(qed_node::coordinator::CoordinatorWorkerArgs),
    #[command(about = "Run the realm edge node")]
    RealmEdge {
        #[command(flatten)]
        config: qed_node::realm::RealmEdgeConfig,
    },
    #[command(about = "Run the realm processor node")]
    RealmProcessor {
        #[command(flatten)]
        config: qed_node::realm::RealmNodeConfig,
    },
    #[command(about = "Run the realm worker node")]
    RealmWorker {
        #[clap(long = "edge-url", default_value = "http://localhost:8546")]
        edge_url: String,
        #[command(flatten)]
        redis_config: qed_node::realm::RedisConfig,
        #[command(flatten)]
        queue_config: qed_node::realm::QueueConfig,
    },
}
