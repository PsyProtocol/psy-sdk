use clap::command;
use clap::Parser;
use clap::Subcommand;

pub mod add_withdrawal;
pub mod args;
pub mod claim_deposit;
pub mod deploy_contract;
pub mod get_public_key;
pub mod l1_deposit;
pub mod produce_block;
pub mod random_wallet;
pub mod register_user;
pub mod sign_hash;
pub mod submit_end_cap_proof;
pub mod token_transfer;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    AddWithdrawal(crate::subcommand::args::AddWithdrawalArgs),
    ClaimDeposit(crate::subcommand::args::ClaimDepositArgs),
    RegisterUser(crate::subcommand::args::RegisterUserArgs),
    TokenTransfer(crate::subcommand::args::TokenTransferArgs),
    L1Deposit(crate::subcommand::args::L1DepositArgs),

    SignHash(crate::subcommand::args::SignHashArgs),
    GetPublicKey(crate::subcommand::args::GetPublicKeyArgs),
    RandomWallet(crate::subcommand::args::RandomWalletArgs),

    DeployContract(crate::subcommand::args::DeployContractArgs),
    ProduceBlock(crate::subcommand::args::ProduceBlockArgs),
    SubmitEndCaproof(crate::subcommand::args::SubmitEndCapArgs),
    // Repl(crate::subcommand::args::RPCReplArgs),
    // ProverRPC(crate::subcommand::args::ProverRPCArgs),
}
