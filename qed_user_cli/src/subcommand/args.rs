use clap::{Args, Parser};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_prover::api::provider::RpcConfig;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Clone, Args)]
pub struct RandomWalletArgs {}

#[derive(Clone, Args)]
pub struct GetPublicKeyArgs {
    /// user private key
    #[clap(long, short)]
    pub private_key: String,
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

