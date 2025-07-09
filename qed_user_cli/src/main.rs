#![cfg(feature = "is_sync")]
mod error;
mod rpc;
mod subcommand;
mod session;

#[cfg(not(target_arch = "wasm32"))]
use shadow_rs::shadow;

#[cfg(not(target_arch = "wasm32"))]
shadow!(build);

use clap::Parser;
use error::Result;
use crate::subcommand::block_state;
use crate::subcommand::deploy_contract;
use crate::subcommand::get_public_key;
use crate::subcommand::produce_block;
use crate::subcommand::random_wallet;
use crate::subcommand::register_user;
use crate::subcommand::submit_end_cap_proof;
use crate::subcommand::Cli;
use crate::subcommand::Commands;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    qed_rollup_utils::setup_logging(cli.log_level)?;
    tracing::info!("qed user cli");
    match cli.command {
        Commands::RegisterUser(args) => register_user::run(args)?,
        Commands::RandomRegisterUserBatch(args) => register_user::run_random(args)?,
        Commands::GetPublicKey(args) => get_public_key::run(args)?,
        Commands::RandomWallet(args) => random_wallet::run(args)?,
        Commands::DeployContract(args) => deploy_contract::run(args)?,
        Commands::ProduceBlock(args) => produce_block::run(args)?,
        Commands::SubmitEndCaproof(args) => submit_end_cap_proof::run(args)?,

        // get block data
        Commands::GetBlockState(block_state_args) => {
            block_state::get_l2_block_state(block_state_args)?
        }
        Commands::GetLatestBlockState(latest_block_state_args) => {
            block_state::get_lastest_block_state(latest_block_state_args)?
        }
        Commands::GetUserId(user_id_args) => block_state::get_user_id(user_id_args)?,
        Commands::GetUserLeaf(user_leaf_args) => block_state::get_user_leaf(user_leaf_args)?,

        // wallet session
        Commands::WalletSession(wallet_session_args) => session::run(wallet_session_args)?,
    }
    Ok(())
}
