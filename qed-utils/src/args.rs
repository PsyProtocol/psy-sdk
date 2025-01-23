use clap::Args;

#[derive(Clone, Args)]
pub struct InterpreterArgs {
    #[clap(short, env, long)]
    pub file: String,
}

#[derive(Clone, Args)]
pub struct CompilerArgs {
    #[clap(short, env, long)]
    pub file: String,
    #[clap(short, env, long)]
    pub function: String,
}
