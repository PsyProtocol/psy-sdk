mod subcommand;

use crate::subcommand::compiler;
use crate::subcommand::interpreter;
use crate::subcommand::test;

use clap::Parser;

use subcommand::Cli;
use subcommand::Commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    qed_utils::setup_env_logger();

    let cli = Cli::parse();
    match cli.command {
        Commands::Interpret(args) => interpreter::run(args).await?,
        Commands::Compile(args) => compiler::run(args)?,
        Commands::Test(args) => test::run(args).await?,
    }
    Ok(())
}
