use clap::command;
use clap::Parser;
use clap::Subcommand;
use qed_prover::local::args::ProveProxyArgs;
use qed_prover::local::args::{ProverArgs, WalletSessionArgs};

pub mod args;
pub mod deploy_contract;
pub mod prove_proxy;
pub mod local_prover;

cfg_if::cfg_if! {
    if #[cfg(all(not(target_arch = "wasm32"), feature = "is_sync"))] {
        pub mod get_public_key;
        pub mod random_wallet;
        pub mod register_user;
        pub mod submit_end_cap_proof;
        pub mod claim_rewards;
    }
}

#[derive(Parser)]
pub struct Cli {
    #[arg(
        long = "log-level",
        default_value = "info",
        help = "Set the log level (error, warn, info, debug, trace)"
    )]
    pub log_level: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    GetPublicKey(crate::subcommand::args::GetPublicKeyArgs),
    RandomWallet(crate::subcommand::args::RandomWalletArgs),
    RegisterUser(crate::subcommand::args::RegisterUserArgs),

    DeployContract(crate::subcommand::args::DeployContractArgs),
    SubmitEndCaproof(crate::subcommand::args::SubmitEndCapArgs),

    GetUserId(crate::subcommand::args::UserIdArgs),
    GetUserLeaf(crate::subcommand::args::UserLeafArgs),

    // Tree commands
    GetUserContractStateTreeRoot(crate::subcommand::args::UserContractStateTreeRootArgs),
    GetUserContractStateTreeLeafHash(crate::subcommand::args::UserContractStateTreeLeafHashArgs),
    GetUserContractStateTreeMerkleProof(crate::subcommand::args::UserContractStateTreeMerkleProofArgs),
    
    GetUserContractTreeRoot(crate::subcommand::args::UserContractTreeRootArgs),
    GetUserContractTreeLeafHash(crate::subcommand::args::UserContractTreeLeafHashArgs),
    GetUserContractTreeMerkleProof(crate::subcommand::args::UserContractTreeMerkleProofArgs),
    
    GetUserRegistrationTreeRoot(crate::subcommand::args::UserRegistrationTreeRootArgs),
    GetUserRegistrationTreeLeafHash(crate::subcommand::args::UserRegistrationTreeLeafHashArgs),
    GetUserRegistrationTreeMerkleProof(crate::subcommand::args::UserRegistrationTreeMerkleProofArgs),
    
    GetUserTreeRoot(crate::subcommand::args::UserTreeRootArgs),
    GetUserTreeLeafHash(crate::subcommand::args::UserTreeLeafHashArgs),
    GetUserTreeMerkleProof(crate::subcommand::args::UserTreeMerkleProofArgs),
    GetUserSubTreeMerkleProof(crate::subcommand::args::UserSubTreeMerkleProofArgs),
    
    GetContractFunctionTreeRoot(crate::subcommand::args::ContractFunctionTreeRootArgs),
    GetContractFunctionTreeLeafHash(crate::subcommand::args::ContractFunctionTreeLeafHashArgs),
    GetContractFunctionTreeMerkleProof(crate::subcommand::args::ContractFunctionTreeMerkleProofArgs),
    
    GetContractTreeRoot(crate::subcommand::args::ContractTreeRootArgs),
    GetContractTreeLeafHash(crate::subcommand::args::ContractTreeLeafHashArgs),
    GetContractTreeMerkleProof(crate::subcommand::args::ContractTreeMerkleProofArgs),
    
    GetDepositTreeRoot(crate::subcommand::args::DepositTreeRootArgs),
    GetDepositTreeLeafHash(crate::subcommand::args::DepositTreeLeafHashArgs),
    GetDepositTreeMerkleProof(crate::subcommand::args::DepositTreeMerkleProofArgs),
    
    GetWithdrawalTreeRoot(crate::subcommand::args::WithdrawalTreeRootArgs),
    GetWithdrawalTreeLeafHash(crate::subcommand::args::WithdrawalTreeLeafHashArgs),
    GetWithdrawalTreeMerkleProof(crate::subcommand::args::WithdrawalTreeMerkleProofArgs),
    
    GetLatestCheckpointTreeRoot(crate::subcommand::args::LatestCheckpointTreeRootArgs),
    GetCheckpointTreeRoot(crate::subcommand::args::CheckpointTreeRootArgs),
    GetCheckpointTreeLeafHash(crate::subcommand::args::CheckpointTreeLeafHashArgs),
    GetCheckpointTreeMerkleProof(crate::subcommand::args::CheckpointTreeMerkleProofArgs),
    
    // Metadata commands
    GetContractLeafData(crate::subcommand::args::ContractLeafDataArgs),
    GetCheckpointLeafData(crate::subcommand::args::CheckpointLeafDataArgs),
    GetContractCodeDefinition(crate::subcommand::args::ContractCodeDefinitionArgs),
    GetLatestL2BlockState(crate::subcommand::args::LatestL2BlockStateArgs),
    GetL2BlockState(crate::subcommand::args::L2BlockStateArgs),

    // session
    WalletSession(WalletSessionArgs),

    // local proving
    LocalProver(ProverArgs),
    ProveProxy(ProveProxyArgs),
    
    // rewards claiming
    ClaimRewards(crate::subcommand::args::ClaimRewardsArgs),
}
