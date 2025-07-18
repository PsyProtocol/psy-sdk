use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct ProverArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[clap(env = "PROVER_LISTEN_ADDR", long, default_value = "0.0.0.0:8888")]
    pub listen_addr: String,
    #[clap(
        long,
        default_value = "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a"
    )]
    pub private_key: String,
    #[clap(
        long,
        default_value = "9f5cb6b51fd293bbc95f94013d65c566d7adeebb7e1cc77c89b9ccd73571b5c0"
    )]
    pub api_key: String,
}

#[derive(Clone, Debug, Parser)]
pub struct ProveProxyArgs {
    #[clap(env = "PROVE_PROXY_LISTEN_ADDR", long, default_value = "0.0.0.0:9999")]
    pub listen_addr: String,
}
