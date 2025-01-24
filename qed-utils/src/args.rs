use clap::Args;

#[derive(Clone, Args)]
pub struct InterpreterArgs {
    #[clap(short, env, long)]
    pub file: String,

    #[clap(short, env, long, num_args = 1.., default_values = &["2", "3"])]
    pub params: Vec<u64>,
}
