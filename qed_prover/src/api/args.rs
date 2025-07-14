use clap::{Args, Parser};
use serde::{Deserialize, Serialize};
use ts_rs::TS;


#[derive(Debug, Clone, Deserialize, Serialize, Parser, TS)]
#[ts(export)]
pub struct ContractCallArgs {
    #[arg(long, default_value = "0", env)]
    pub contract_id: u64,
    #[arg(long, default_value = "main", env)]
    pub method_name: String,
    #[arg(long, default_value = "[]", env)]
    pub inputs: Vec<u64>,
}

pub fn parse_contract_call_args(s: &str) -> anyhow::Result<Vec<ContractCallArgs>> {
    serde_json::from_str(s).map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))
}

#[derive(Clone, Args)]
pub struct WalletSessionArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(
        long,
        default_value = "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a",
        env
    )]
    pub private_key: String,
    #[clap(env, long, default_value = "contract_call.json", env)]
    pub contract_calls: String,
}
