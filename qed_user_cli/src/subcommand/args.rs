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

// Tree-related args
#[derive(Clone, Args)]
pub struct UserContractStateTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
}

#[derive(Clone, Args)]
pub struct UserContractStateTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
    #[arg(long, env)]
    pub height: u8,
    #[arg(long, env)]
    pub leaf_id: u64,
}

#[derive(Clone, Args)]
pub struct UserContractStateTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
    #[arg(long, env)]
    pub height: u8,
    #[arg(long, env)]
    pub leaf_id: u64,
}

#[derive(Clone, Args)]
pub struct UserContractTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
}

#[derive(Clone, Args)]
pub struct UserContractTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
}

#[derive(Clone, Args)]
pub struct UserContractTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
}

#[derive(Clone, Args)]
pub struct UserRegistrationTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct UserRegistrationTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub leaf_index: u64,
}

#[derive(Clone, Args)]
pub struct UserRegistrationTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub leaf_index: u64,
}

#[derive(Clone, Args)]
pub struct UserTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct UserTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
}

#[derive(Clone, Args)]
pub struct UserTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub user_id: u64,
}

#[derive(Clone, Args)]
pub struct UserSubTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub root_level: u8,
    #[arg(long, env)]
    pub leaf_level: u8,
    #[arg(long, env)]
    pub leaf_index: u64,
}

#[derive(Clone, Args)]
pub struct ContractFunctionTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
}

#[derive(Clone, Args)]
pub struct ContractFunctionTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
    #[arg(long, env)]
    pub function_id: u32,
}

#[derive(Clone, Args)]
pub struct ContractFunctionTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
    #[arg(long, env)]
    pub function_id: u32,
}

#[derive(Clone, Args)]
pub struct ContractTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct ContractTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
}

#[derive(Clone, Args)]
pub struct ContractTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub contract_id: u32,
}

#[derive(Clone, Args)]
pub struct DepositTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct DepositTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub deposit_id: u32,
}

#[derive(Clone, Args)]
pub struct DepositTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub deposit_id: u32,
}

#[derive(Clone, Args)]
pub struct WithdrawalTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct WithdrawalTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub withdrawal_id: u32,
}

#[derive(Clone, Args)]
pub struct WithdrawalTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub withdrawal_id: u32,
}

#[derive(Clone, Args)]
pub struct LatestCheckpointTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
}

#[derive(Clone, Args)]
pub struct CheckpointTreeRootArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct CheckpointTreeLeafHashArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub leaf_checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct CheckpointTreeMerkleProofArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
    #[arg(long, env)]
    pub leaf_checkpoint_id: u64,
}

// Metadata-related args
#[derive(Clone, Args)]
pub struct ContractLeafDataArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub contract_id: u64,
}

#[derive(Clone, Args)]
pub struct CheckpointLeafDataArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

#[derive(Clone, Args)]
pub struct ContractCodeDefinitionArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub contract_id: u64,
}

#[derive(Clone, Args)]
pub struct LatestL2BlockStateArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
}

#[derive(Clone, Args)]
pub struct L2BlockStateArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, env)]
    pub checkpoint_id: u64,
}

