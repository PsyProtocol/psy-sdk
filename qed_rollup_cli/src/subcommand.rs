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
    CoordinatorEdge(qed_coordinator_node::CoordinatorEdgeArgs),
    #[command(about = "Run the coordinator processor node")]
    CoordinatorProcessor(qed_coordinator_node::CoordinatorProcessorArgs),
    #[command(about = "Run the coordinator worker node")]
    CoordinatorWorker(qed_coordinator_node::CoordinatorWorkerArgs),
    #[command(about = "Run the realm edge node")]
    RealmEdge {
        #[command(flatten)]
        config: qed_realm_node::RealmEdgeConfig,
    },
    #[command(about = "Run the realm processor node")]
    RealmProcessor {
        #[command(flatten)]
        config: qed_realm_node::RealmNodeConfig,
    },
    #[command(about = "Run the realm worker node")]
    RealmWorker {
        #[command(flatten)]
        redis_config: qed_realm_node::RedisConfig,
        #[command(flatten)]
        queue_config: qed_realm_node::QueueConfig,
    },
}
