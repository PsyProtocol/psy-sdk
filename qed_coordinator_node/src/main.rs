use anyhow::Ok;
use clap::Parser;
use subcommand::Cli;

mod processor;
mod subcommand;
mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        subcommand::Commands::Processor(args) => processor::run(args).await?,
        subcommand::Commands::Worker(args) => worker::run(args).await?,
    }
    Ok(())
}
