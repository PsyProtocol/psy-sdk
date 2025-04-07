mod subcommand;

use crate::subcommand::compiler;
use crate::subcommand::interpreter;
use crate::subcommand::test;

use clap::Parser;

use subcommand::lsp;
use subcommand::Cli;
use subcommand::Commands;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    qed_utils::setup_env_logger();

    let cli = Cli::parse();
    match cli.command {
        Commands::Interpret(args) => interpreter::run(args)?,
        Commands::Compile(args) => compiler::run(args)?,
        Commands::Test(args) => test::run(args)?,
        Commands::LSP(args) => lsp::run(args)?,
    }
    Ok(())
}
