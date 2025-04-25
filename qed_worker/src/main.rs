use anyhow::Result;
use clap::{Parser, Subcommand};
use qed_worker::config::LogConfig;
use qed_worker::config::{setup_logging, WorkerConfig};
use qed_worker::WorkerState;
use qed_worker::{CoordinatorWorker, EdgeWorker, Worker};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    info!("Starting QED Worker");
    // Load configuration from environment variables
    let cli = Cli::parse();

    // Initialize logging
    setup_logging(&cli.log)?;

    let worker: Box<dyn Worker> = match cli.command {
        Commands::Edge { config } => Box::new(EdgeWorker::from(WorkerState::new(config).await?)),
        Commands::Coordinator { config } => {
            Box::new(CoordinatorWorker::from(WorkerState::new(config).await?))
        }
    };

    tokio::select! {
        err = worker.run() => {
            error!("Worker finished with error: {:?}", err);
        }
        _ = tokio::signal::ctrl_c() => {
            warn!("Received Ctrl+C, shutting down...");
        }
    }

    Ok(())
}

#[derive(Parser, Deserialize, Serialize, Debug)]
#[command(author, version, about = "QED Worker Service")]
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
        config: WorkerConfig,
    },
    Coordinator {
        #[command(flatten)]
        config: WorkerConfig,
    },
}
