use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct ContractCallArgs {
    pub contract_id: u64,
    pub method_name: String,
    pub inputs: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletKeyPair {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: u64,
    pub rpc_url: Vec<String>,
}