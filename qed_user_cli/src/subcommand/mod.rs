use clap::command;
use clap::Parser;
use clap::Subcommand;
use qed_prover::api::args::WalletSessionArgs;
use qed_prover::local::args::ProverArgs;

pub mod args;
pub mod deploy_contract;

cfg_if::cfg_if! {
    if #[cfg(all(not(target_arch = "wasm32"), feature = "is_sync"))] {
        pub mod get_public_key;
        pub mod random_wallet;
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
    GetPublicKey(crate::subcommand::args::GetPublicKeyArgs),
    RandomWallet(crate::subcommand::args::RandomWalletArgs),

    DeployContract(crate::subcommand::args::DeployContractArgs),
    SubmitEndCaproof(crate::subcommand::args::SubmitEndCapArgs),

    GetUserId(crate::subcommand::args::UserIdArgs),
    GetUserLeaf(crate::subcommand::args::UserLeafArgs),

    // session
    WalletSession(WalletSessionArgs),
    LocalProver(ProverArgs),
}
