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

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    match cli.command {
        Commands::AddWithdrawal(args) => add_withdrawal::run(args).await?,
        Commands::ClaimDeposit(args) => claim_deposit::run(args).await?,
        Commands::RegisterUser(args) => register_user::run(args).await?,
        Commands::TokenTransfer(args) => token_transfer::run(args).await?,
        Commands::L1Deposit(args) => l1_deposit::run(args).await?,
        Commands::SignHash(args) => sign_hash::run(args).await?,
        Commands::GetPublicKey(args) => get_public_key::run(args).await?,
        Commands::RandomWallet(args) => random_wallet::run(args).await?,
        Commands::DeployContract(args) => deploy_contract::run(args).await?,
        Commands::ProduceBlock(args) => produce_block::run(args).await?,
        Commands::SubmitEndCaproof(args) => submit_end_cap_proof::run(args).await?,
    }
    Ok(())
}
