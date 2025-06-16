use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod add_withdrawal;
pub mod args;
pub mod block_state;
pub mod claim_deposit;
pub mod deploy_contract;
pub mod get_public_key;
pub mod l1_deposit;

#[cfg(not(target_arch = "wasm32"))]
pub mod lps;
pub mod produce_block;
pub mod random_wallet;
pub mod register_user;
pub mod sign_hash;
pub mod submit_end_cap_proof;
pub mod token_transfer;
pub mod utils;

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
    AddWithdrawal(crate::subcommand::args::AddWithdrawalArgs),
    ClaimDeposit(crate::subcommand::args::ClaimDepositArgs),
    RegisterUser(crate::subcommand::args::RegisterUserArgs),
    RandomRegisterUserBatch(crate::subcommand::args::RandomArgs),
    TokenTransfer(crate::subcommand::args::TokenTransferArgs),
    L1Deposit(crate::subcommand::args::L1DepositArgs),

    SignHash(crate::subcommand::args::SignHashArgs),
    GetPublicKey(crate::subcommand::args::GetPublicKeyArgs),
    RandomWallet(crate::subcommand::args::RandomWalletArgs),

    DeployContract(crate::subcommand::args::DeployContractArgs),
    ProduceBlock(crate::subcommand::args::ProduceBlockArgs),
    SubmitEndCaproof(crate::subcommand::args::SubmitEndCapArgs),
    // Lps(crate::subcommand::args::LPSArgs),
    // Repl(crate::subcommand::args::RPCReplArgs),
    // ProverRPC(crate::subcommand::args::ProverRPCArgs),

    // get block data
    GetBlockState(crate::subcommand::args::BlockStateArgs),
    GetLatestBlockState(crate::subcommand::args::LatestBlockStateArgs),
    GetUserId(crate::subcommand::args::UserIdArgs),
    GetUserLeaf(crate::subcommand::args::UserLeafArgs),

    // session
    WalletSession(crate::subcommand::args::WalletSessionArgs),
}
