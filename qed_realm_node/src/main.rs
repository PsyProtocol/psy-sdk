use anyhow::Result;
use clap::{Parser, Subcommand};
use qed_realm_node::config::{LogConfig, RealmEdgeConfig};
use qed_realm_node::{
    config::{setup_logging, RealmNodeConfig},
    edge::start_realm_edge_node,
    start_realm_processor_node,
};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from environment variables
    let cli = Cli::parse();

    // Initialize logging
    setup_logging(&cli.log)?;

    match cli.command {
        Commands::Edge { config } => start_realm_edge_node(config).await?,
        Commands::Processor { config } => start_realm_processor_node(config).await?,
    }

    Ok(())
}

#[derive(Parser, Deserialize, Serialize, Debug)]
#[command(author, version, about = "QED Realm Node Service")]
struct Cli {
    /// Log configuration
    #[command(flatten)]
    pub log: LogConfig,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Deserialize, Serialize, Debug)]
enum Commands {
    Edge {
        #[command(flatten)]
        config: RealmEdgeConfig,
    },
    Processor {
        #[command(flatten)]
        config: RealmNodeConfig,
    },
}
