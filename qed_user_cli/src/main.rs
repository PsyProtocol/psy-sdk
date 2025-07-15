#![cfg(feature = "is_sync")]
mod error;
mod subcommand;

use qed_prover::session;
#[cfg(not(target_arch = "wasm32"))]
use shadow_rs::shadow;

#[cfg(not(target_arch = "wasm32"))]
shadow!(build);

use clap::Parser;
use error::Result;
use crate::subcommand::deploy_contract;
use crate::subcommand::get_public_key;
use crate::subcommand::random_wallet;
use crate::subcommand::submit_end_cap_proof;
use crate::subcommand::Cli;
use crate::subcommand::Commands;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    qed_rollup_utils::setup_logging(cli.log_level)?;
    tracing::info!("qed user cli");
    match cli.command {
        Commands::GetPublicKey(args) => get_public_key::run(args)?,
        Commands::RandomWallet(args) => random_wallet::run(args)?,
        Commands::DeployContract(args) => deploy_contract::run(args)?,
        Commands::SubmitEndCaproof(args) => submit_end_cap_proof::run(args)?,

        // get block data
        Commands::GetUserId(user_id_args) => {
            use crate::subcommand::args::UserIdArgs;
            use qed_prover::api::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&user_id_args.rpc_config)?;
            let user_id = provider.get_user_id(user_id_args.pub_key)?;
            println!("user_id: {}", user_id);
        }
        Commands::GetUserLeaf(user_leaf_args) => {
            use crate::subcommand::args::UserLeafArgs;
            use qed_prover::api::provider::RpcProvider;
            use qed_crypto::hash::traits::qhashable::QFieldHashable;
            use qed_data::config::store_config::QEDHasher;
            use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&user_leaf_args.rpc_config)?;
            let user_id = provider.get_user_id(user_leaf_args.pub_key)?;
            let user_leaf_data = provider.get_user_leaf_data(user_leaf_args.checkpoint_id, user_id)?;
            println!("user_leaf_data: {}", serde_json::to_string_pretty(&user_leaf_data)?);
            println!("user_leaf_hash: {}", user_leaf_data.qfhash::<QEDHasher>().to_string());
        }

        // wallet session
        Commands::WalletSession(wallet_session_args) => session::run(wallet_session_args)?,
        Commands::LocalProver(prover_args) => qed_prover::run_server(prover_args).await?,
    }
    Ok(())
}
