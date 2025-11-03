use std::collections::HashMap;

use clap::{Args, Parser, ValueEnum};
use plonky2::hash::hash_types::RichField;
use psy_common::{data::qhashout::QHashOut, job::id::QProvingJobDataID};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, Serialize, Parser, TS)]
#[ts(export)]
pub struct ContractCallArgs {
    #[arg(long, default_value = "0", env)]
    pub contract_id: u64,
    #[arg(long, default_value = "main", env)]
    pub method_name: String,
    #[arg(long, env)]
    pub inputs: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, ValueEnum, Deserialize, Serialize, Parser, TS)]
pub enum SignType {
    #[clap(name = "zk")]
    ZKSign,
    #[clap(name = "secp256k1")]
    SECP256K1Sign,
    #[clap(name = "software-defined")]
    SoftwareDefinedSign,
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone, Hash, Eq, Debug)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SignData<F: RichField> {
    pub fingerprint: QHashOut<F>,
    pub sign_contract_id: u64,
    pub sign_inputs: Vec<u64>,
}

pub fn parse_contract_call_args(s: &str) -> anyhow::Result<Vec<ContractCallArgs>> {
    serde_json::from_str(s).map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))
}

#[derive(Clone, Args, Serialize, Deserialize)]
pub struct WalletSessionArgs {
    #[clap(env, long, default_value = "config.json", env)]
    pub rpc_config: String,
    #[arg(long, short, default_value = "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a", env)]
    pub private_key: String,
    #[clap(env, long, default_value = "contract_call.json", env)]
    pub contract_calls: String,
    #[clap(env, long, default_value = "zk", env)]
    pub sign_type: SignType,
    #[arg(long, default_value = "0", env)]
    pub contract_id: u64,
    #[clap(long)]
    pub sign_inputs: Vec<u64>,
}

#[derive(Clone, Debug, Parser)]
pub struct ProverArgs {
    #[clap(env, long, default_value = "config.json", env)]
    pub rpc_config: String,
    #[clap(env = "PROVER_LISTEN_ADDR", long, default_value = "0.0.0.0:8888")]
    pub listen_addr: String,
    #[clap(long, default_value = "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a")]
    pub private_key: String,
    #[clap(long, default_value = "9f5cb6b51fd293bbc95f94013d65c566d7adeebb7e1cc77c89b9ccd73571b5c0")]
    pub api_key: String,
}

#[derive(Clone, Debug, Parser)]
pub struct ProveProxyArgs {
    #[clap(env = "PROVE_PROXY_LISTEN_ADDR", long, default_value = "0.0.0.0:9999")]
    pub listen_addr: String,
    #[clap(env, long, default_value = "config.json", env)]
    pub rpc_config: String,
}

pub use psy_common::{JobInfo, JobLocation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmJobData {
    pub id: u32,
    pub checkpoints: HashMap<u64, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerJobTracker {
    pub coordinator: HashMap<u64, Vec<String>>,
    pub realms: Vec<RealmJobData>,
}
