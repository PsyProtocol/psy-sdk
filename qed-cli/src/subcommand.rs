use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod interpreter;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Interpret(qed_utils::args::InterpreterArgs),
}
