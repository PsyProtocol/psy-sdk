mod subcommand;

use crate::subcommand::compiler;
use crate::subcommand::interpreter;

use clap::Parser;

use subcommand::Cli;
use subcommand::Commands;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    qed_utils::setup_env_logger();

    let cli = Cli::parse();
    match cli.command {
        Commands::Interpret(args) => interpreter::run(args)?,
        Commands::Compile(args) => compiler::run(args)?,
    }
    Ok(())
}
