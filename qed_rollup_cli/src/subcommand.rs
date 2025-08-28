use clap::command;
use clap::ArgAction;
use clap::Parser;
use clap::Subcommand;

pub mod coordinator_edge;
pub mod coordinator_processor;
pub mod realm_edge;
pub mod realm_processor;
pub mod worker;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Run the coordinator edge node")]
    CoordinatorEdge(qed_node::coordinator::CoordinatorEdgeArgs),
    #[command(about = "Run the coordinator processor node")]
    CoordinatorProcessor(qed_node::coordinator::CoordinatorProcessorArgs),
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
    #[command(about = "Run the worker node")]
    Worker {
        #[arg(long = "config", default_value = "./config.json", help = "Path to config.json file")]
        config: String,

        #[arg(long = "private-key", env = "PRIVATE_KEY", help = "Private key hex string")]
        private_key: Option<String>,

        #[arg(long = "keystore-path", env = "KEYSTORE_PATH", help = "Path to wallet keystore file")]
        keystore_path: Option<String>,

        #[arg(long = "wallet-password", env = "WALLET_PASSWORD", help = "Wallet password")]
        wallet_password: Option<String>,
    },
}
