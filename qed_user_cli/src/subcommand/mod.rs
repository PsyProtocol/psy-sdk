use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod args;
pub mod utils;
pub mod deploy_contract;

cfg_if::cfg_if! {
    if #[cfg(all(not(target_arch = "wasm32"), feature = "is_sync"))] {
        pub mod block_state;
        pub mod get_public_key;
        pub mod produce_block;
        pub mod random_wallet;
        pub mod register_user;
        pub mod submit_end_cap_proof;
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
    RegisterUser(crate::subcommand::args::RegisterUserArgs),
    RandomRegisterUserBatch(crate::subcommand::args::RandomArgs),

    GetPublicKey(crate::subcommand::args::GetPublicKeyArgs),
    RandomWallet(crate::subcommand::args::RandomWalletArgs),

    DeployContract(crate::subcommand::args::DeployContractArgs),
    ProduceBlock(crate::subcommand::args::ProduceBlockArgs),
    SubmitEndCaproof(crate::subcommand::args::SubmitEndCapArgs),

    // get block data
    GetBlockState(crate::subcommand::args::BlockStateArgs),
    GetLatestBlockState(crate::subcommand::args::LatestBlockStateArgs),
    GetUserId(crate::subcommand::args::UserIdArgs),
    GetUserLeaf(crate::subcommand::args::UserLeafArgs),

    // session
    WalletSession(crate::subcommand::args::WalletSessionArgs),
}
