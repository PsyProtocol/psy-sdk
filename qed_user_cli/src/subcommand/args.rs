use clap::Args;
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

    #[clap(long, short, default_value = "http://127.0.0.1:3000", env)]
    pub rpc_config_path: String,
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
    #[clap(long, short, default_value = "http://127.0.0.1:3000", env)]
    pub rpc_config_path: String,

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
    #[clap(long, short, default_value = "http://127.0.0.1:3000", env)]
    pub rpc_config_path: String,

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
    #[clap(long, short, default_value = "http://127.0.0.1:3000", env)]
    pub rpc_config_path: String,

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
    #[clap(long, short, default_value = "rpc.config", env)]
    pub rpc_config_path: String,
    /// user private key
    #[clap(long, short, default_value = "")]
    pub private_key: String,
}

#[derive(Clone, Args)]
pub struct TokenTransferArgs {
    #[clap(long, short, default_value = "http://127.0.0.1:3000", env)]
    pub rpc_config_path: String,

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
    #[clap(long, short, default_value = "rpc.config", env)]
    pub rpc_config_path: String,
}

#[derive(Clone, Args)]
pub struct DeployContractArgs {
    #[clap(long, short, default_value = "rpc.config", env)]
    pub rpc_config_path: String,
    #[clap(long, short, env)]
    pub private_key: String,
    #[clap(long, short)]
    pub contract_path: String,
}

#[derive(Clone, Args)]
pub struct SubmitEndCapArgs {
    #[clap(long, short, default_value = "rpc.config", env)]
    pub rpc_config_path: String,
    #[clap(long, short)]
    pub private_key: String,
    #[clap(long, short)]
    pub contract_call_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractCallArgs {
    pub contract_id: u64,
    pub method_name: String,
    pub inputs: Vec<u64>,
}
