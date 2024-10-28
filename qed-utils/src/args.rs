use clap::Args;

#[derive(Clone, Args)]
pub struct InterpreterArgs {
    #[clap(short, env, long)]
    pub file: String,
}
