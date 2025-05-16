use crate::rpc::provider::RpcConfig;
use clap::{Args, Parser};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};

#[derive(Clone, Args)]
pub struct RandomWalletArgs {}

#[derive(Clone, Args)]
pub struct RPCReplArgs {
    #[clap(
        env,
        long,
        default_value = "http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=dogeRegtest",
        env
    )]
    pub bitcoin_rpc: String,
    #[clap(env, long, default_value = "http://localhost:1337/api", env)]
    pub electrs_api: String,

    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
}

#[derive(Clone, Args)]
pub struct ProverRPCArgs {
    #[clap(long, short, default_value = "0.0.0.0:1447", env)]
    pub prover_rpc_address: String,
    #[clap(long, short, default_value = "")]
    pub api_key: String,
}

#[derive(Clone, Args)]
pub struct GetPublicKeyArgs {
    /// user private key
    #[clap(long, short)]
    pub private_key: String,
}

#[derive(Clone, Args)]
pub struct SignHashArgs {
    /// user private key
    #[clap(long, short)]
    pub private_key: String,

    /// action hash to sign
    #[clap(long, short)]
    pub action_hash: String,

    /// output file path for the proof
    #[clap(long, short)]
    pub output: String,
}

#[derive(Clone, Args)]
pub struct L1DepositArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,

    #[clap(long, short)]
    pub private_key: String,

    #[clap(long, short, default_value = "")]
    pub deposit_address: String,

    #[clap(long, short)]
    pub amount: u64,

    #[clap(
        env,
        long,
        default_value = "http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=dogeRegtest",
        env
    )]
    pub bitcoin_rpc: String,
    #[clap(env, long, default_value = "http://localhost:1337/api", env)]
    pub electrs_api: String,
}

#[derive(Clone, Args)]
pub struct AddWithdrawalArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,

    #[clap(long, default_value = "dogeregtest", env)]
    pub network: String,

    #[clap(long, short)]
    pub private_key: String,

    #[clap(long, short)]
    pub user_id: u64,

    #[clap(long, short)]
    pub value: u64,

    #[clap(long, short)]
    pub nonce: u64,

    // #[clap(long, short)]
    // pub destination_type: u8,
    #[clap(long, short)]
    pub destination: String,
}

#[derive(Clone, Args)]
pub struct ClaimDepositArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,

    #[clap(long, short)]
    pub private_key: String,

    #[clap(long, default_value = "dogeregtest", env)]
    pub network: String,

    /// l1deposit
    #[clap(long, short)]
    pub txid: String,

    #[clap(long, short)]
    pub user_id: u64,
}

#[derive(Clone, Args)]
pub struct RegisterUserArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    /// user private key
    #[clap(
        long,
        short,
        default_value = "f93ee5497d94c7d216bb5daaf77a60a4903cb7c69b752c3e1a24753691505998"
    )]
    pub private_key: String,
}

#[derive(Clone, Args)]
pub struct TokenTransferArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,

    #[clap(long, default_value = "dogeregtest", env)]
    pub network: String,

    #[clap(long, short)]
    pub private_key: String,

    #[clap(long, short)]
    pub value: u64,

    #[clap(long, short)]
    pub from: u64,

    #[clap(long, short)]
    pub to: u64,

    #[clap(long, short)]
    pub nonce: u64,
}

#[derive(Clone, Args)]
pub struct ProduceBlockArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
}

#[derive(Clone, Args)]
pub struct DeployContractArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[clap(long, env)]
    pub private_key: String,
    #[clap(long)]
    pub contract_path: String,
}

#[derive(Clone, Args, Serialize, Deserialize)]
pub struct SubmitEndCapArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[clap(long, short)]
    pub private_key: String,
    #[arg(long, default_value = "0", env)]
    pub contract_id: u64,
    #[arg(long, default_value = "main", env)]
    pub method_name: String,
    #[arg(long, default_value = "[]", env)]
    pub inputs: Vec<u64>,
}

#[derive(Clone, Args)]
pub struct LPSArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[clap(long, short, default_values = &["./db/coordinator", "./db/realm"], env)]
    pub store_config_path: Vec<String>,
    #[clap(long, short)]
    pub private_key: String,
    #[clap(long, short)]
    pub contract_call_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Parser)]
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
pub struct BlockStateArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, default_value = "0", env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct LatestBlockStateArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
}

#[derive(Clone, Args)]
pub struct UserIdArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(
        long,
        default_value = "0d47fda4480f045506b085ba6921fc86d8cc6feb1b533292db4b1a3af8f89eab",
        env
    )]
    pub pub_key: QHashOut<GoldilocksField>,
}

#[derive(Clone, Args)]
pub struct UserLeafArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(
        long,
        default_value = "0d47fda4480f045506b085ba6921fc86d8cc6feb1b533292db4b1a3af8f89eab",
        env
    )]
    pub pub_key: QHashOut<GoldilocksField>,
    #[arg(long, default_value = "100", env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct WalletSessionArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(
        long,
        default_value = "0d47fda4480f045506b085ba6921fc86d8cc6feb1b533292db4b1a3af8f89eab",
        env
    )]
    pub private_key: String,
    #[clap(env, long, default_value = "contract_call.json", env)]
    pub contract_calls: String,
}

#[derive(Clone, Args)]
pub struct RandomArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[clap(long, default_value = "128", env)]
    pub user_per_block: u64,
    #[clap(long, default_value = "4096", env)]
    pub total_user: u64,
    #[clap(long, default_value = "3", env)]
    pub interval: u64,
}
