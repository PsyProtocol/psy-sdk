use clap::Args;

#[derive(Clone, Args)]
pub struct InterpreterArgs {
    #[clap(short, env, long)]
    pub file: String,

    #[clap(short, env, long, default_value = None)]
    pub contract_name: Option<String>,

    #[clap(short, env, long, default_value = "main")]
    pub method_name: String,

    #[clap(short, env, long, num_args = 1..)]
    pub params: Vec<u64>,
}

#[derive(Clone, Args)]
pub struct CompilerArgs {
    #[clap(short, env, long)]
    pub file: String,

    #[clap(short, env, long, default_value = None)]
    pub contract_name: Option<String>,

    #[clap(short, env, long, num_args = 1.., default_values = &["main"])]
    pub method_names: Vec<String>,
}

#[derive(Clone, Args)]
pub struct TestArgs {
    #[clap(short, env, long)]
    pub file: String,
}
