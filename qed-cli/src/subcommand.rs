use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod compiler;
pub mod interpreter;
pub mod lsp;
pub mod test;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Interpret(qed_utils::InterpreterArgs),
    Compile(qed_utils::CompilerArgs),
    Test(qed_utils::TestArgs),
    LSP(qed_utils::LspArgs),
}
