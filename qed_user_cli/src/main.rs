mod constant;
mod error;
mod rpc;
mod subcommand;

use shadow_rs::shadow;

shadow!(build);

use clap::Parser;
use error::Result;
use subcommand::deploy_contract;
use subcommand::produce_block;

use crate::subcommand::add_withdrawal;
use crate::subcommand::block_state;
use crate::subcommand::claim_deposit;
use crate::subcommand::get_public_key;
use crate::subcommand::l1_deposit;
use crate::subcommand::random_wallet;
use crate::subcommand::register_user;
use crate::subcommand::sign_hash;
use crate::subcommand::submit_end_cap_proof;
use crate::subcommand::token_transfer;
use crate::subcommand::Cli;
use crate::subcommand::Commands;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    match cli.command {
        Commands::AddWithdrawal(args) => add_withdrawal::run(args)?,
        Commands::ClaimDeposit(args) => claim_deposit::run(args)?,
        Commands::RegisterUser(args) => register_user::run(args)?,
        Commands::TokenTransfer(args) => token_transfer::run(args)?,
        Commands::L1Deposit(args) => l1_deposit::run(args)?,
        Commands::SignHash(args) => sign_hash::run(args)?,
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
    }
    Ok(())
}
