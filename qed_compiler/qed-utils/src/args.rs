use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct InterpreterArgs {
    #[clap(short, env, long)]
    pub file: String,

    #[clap(short, env, long, default_value = None)]
    pub contract_name: Option<String>,

    #[clap(short, env, long, num_args = 1.., default_values = &["main"])]
    pub method_names: Vec<String>,

    #[clap(short, env, long, value_parser = parse_vec_u64, num_args = 0..)]
    pub parameters: Vec<Vec<u64>>,
}

fn parse_vec_u64(s: &str) -> Result<Vec<u64>, String> {
    s.split(',')
        .map(|num| num.parse::<u64>().map_err(|e| e.to_string()))
        .collect()
}

#[derive(Clone, Debug, Args)]
pub struct CompilerArgs {
    #[clap(short, env, long)]
    pub file: String,

    #[clap(short, env, long, default_value = None)]
    pub contract_name: Option<String>,

    #[clap(short, env, long, num_args = 1.., default_values = &["main"])]
    pub method_names: Vec<String>,
}

#[derive(Clone, Debug, Args)]
pub struct TestArgs {
    #[clap(short, env, long)]
    pub file: String,
}
