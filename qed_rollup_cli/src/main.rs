mod subcommand;

use clap::Parser;

use crate::subcommand::coordinator_edge;
use crate::subcommand::coordinator_processor;
use crate::subcommand::realm_edge;
use crate::subcommand::realm_processor;
use crate::subcommand::worker;

use crate::subcommand::Cli;
use crate::subcommand::Commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    qed_rollup_utils::setup_logging(cli.log_level)?;
    match cli.command {
        Commands::CoordinatorEdge(args) => {
            coordinator_edge::run(args).await?;
        }
        Commands::CoordinatorProcessor(args) => {
            coordinator_processor::run(args).await?;
        }
        Commands::RealmEdge { config } => {
            realm_edge::run(config).await?;
        }
        Commands::RealmProcessor { config } => {
            realm_processor::run(config).await?;
        }
        Commands::Worker { edge_url } => {
            worker::run(edge_url).await?;
        }
    };
    Ok::<_, anyhow::Error>(())
}
