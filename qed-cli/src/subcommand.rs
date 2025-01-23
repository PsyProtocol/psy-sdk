use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod compiler;
pub mod interpreter;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Interpret(qed_utils::InterpreterArgs),
    Compile(qed_utils::CompilerArgs),
}
