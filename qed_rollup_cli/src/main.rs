mod subcommand;

use clap::Parser;

use crate::subcommand::coordinator_edge;
use crate::subcommand::coordinator_processor;
use crate::subcommand::coordinator_worker;
use crate::subcommand::realm_edge;
use crate::subcommand::realm_processor;

use crate::subcommand::Cli;
use crate::subcommand::Commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();


    let cli = Cli::parse();
    match cli.command {
        Commands::CoordinatorEdge(args) => {
            coordinator_edge::run(args).await?;
        },
        Commands::CoordinatorWorker(args) => {
            coordinator_worker::run(args).await?;
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
    };
    Ok::<_, anyhow::Error>(())



}
