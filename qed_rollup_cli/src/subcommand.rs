use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod coordinator_edge;
pub mod coordinator_processor;
pub mod coordinator_worker;
pub mod realm_edge;
pub mod realm_processor;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    CoordinatorEdge(qed_coordinator_node::CoordinatorEdgeArgs),
    CoordinatorProcessor(qed_coordinator_node::CoordinatorProcessorArgs),
    CoordinatorWorker(qed_coordinator_node::CoordinatorWorkerArgs),
    RealmEdge {
        #[command(flatten)]
        config: qed_realm_node::RealmEdgeConfig,
    },
    RealmProcessor {
        #[command(flatten)]
        config: qed_realm_node::RealmNodeConfig,
    },
}
