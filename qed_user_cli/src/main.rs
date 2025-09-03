#![cfg(feature = "is_sync")]
mod error;
mod subcommand;

use qed_prover::local::native::prove_proxy;
use qed_prover::session;
#[cfg(not(target_arch = "wasm32"))]
use shadow_rs::shadow;

#[cfg(not(target_arch = "wasm32"))]
shadow!(build);

use clap::Parser;
use error::Result;
use crate::subcommand::claim_rewards;
use crate::subcommand::deploy_contract;
use crate::subcommand::get_public_key;
use crate::subcommand::random_wallet;
use crate::subcommand::wallet;
use crate::subcommand::register_user;
use crate::subcommand::submit_end_cap_proof;
use crate::subcommand::Cli;
use crate::subcommand::Commands;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    qed_common::setup_logging()?;
    tracing::info!("qed user cli");
    match cli.command {
        Commands::GetPublicKey(args) => get_public_key::run(args)?,
        Commands::RandomWallet(args) => random_wallet::run(args)?,
        Commands::Wallet(args) => wallet::run(args)?,
        Commands::RegisterUser(args) => register_user::run(args)?,
        Commands::DeployContract(args) => deploy_contract::run(args)?,
        Commands::SubmitEndCaproof(args) => submit_end_cap_proof::run(args)?,
        Commands::ClaimRewards(args) => claim_rewards::run(args)?,

        // get block data
        Commands::GetUserId(user_id_args) => {
            use crate::subcommand::args::UserIdArgs;
            use qed_prover::local::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&user_id_args.rpc_config)?;
            let user_id = provider.get_user_id(user_id_args.pub_key)?;
            println!("user_id: {}", user_id);
        }
        Commands::GetUserLeaf(user_leaf_args) => {
            use crate::subcommand::args::UserLeafArgs;
            use qed_prover::local::provider::RpcProvider;
            use qed_crypto::hash::traits::qhashable::QFieldHashable;
            use qed_data::config::store_config::QEDHasher;
            use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;

            let provider = RpcProvider::new_with_config_path(&user_leaf_args.rpc_config)?;

            let (user_id, query_method) = match (&user_leaf_args.pub_key, &user_leaf_args.user_id) {
                (Some(pub_key), None) => {
                    // Query by public key - get user_id from coordinator first
                    let user_id = provider.get_user_id(*pub_key)?;
                    (user_id, "public_key")
                },
                (None, Some(user_id)) => {
                    // Query by user_id directly - use provided user_id
                    (*user_id, "user_id")
                },
                (Some(_), Some(_)) => {
                    return Err(anyhow::format_err!("Cannot specify both --pub-key and --user-id"));
                },
                (None, None) => {
                    return Err(anyhow::format_err!("Must specify either --pub-key or --user-id"));
                }
            };

            let user_leaf_data = provider.get_user_leaf_data(user_leaf_args.checkpoint_id, user_id)?;
            println!("Query method: {}", query_method);
            println!("Resolved user_id: {}", user_id);
            println!("user_leaf_data: {}", serde_json::to_string_pretty(&user_leaf_data)?);
            println!("user_leaf_hash: {}", user_leaf_data.qfhash::<QEDHasher>().to_string());
        }

        // Tree commands
        Commands::GetUserContractStateTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_user_contract_state_tree_root(args.checkpoint_id, args.user_id, args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserContractStateTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_user_contract_state_tree_leaf_hash(args.checkpoint_id, args.user_id, args.contract_id, args.height, args.leaf_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserContractStateTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_user_contract_state_tree_merkle_proof(args.checkpoint_id, args.user_id, args.contract_id, args.height, args.leaf_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserContractTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_user_contract_tree_root(args.checkpoint_id, args.user_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserContractTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_user_contract_tree_leaf_hash(args.checkpoint_id, args.user_id, args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserContractTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_user_contract_tree_merkle_proof(args.checkpoint_id, args.user_id, args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserRegistrationTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_user_registration_tree_root(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserRegistrationTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_user_registration_tree_leaf_hash(args.checkpoint_id, args.leaf_index)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserRegistrationTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_user_registration_tree_merkle_proof(args.checkpoint_id, args.leaf_index)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_user_tree_root(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_user_tree_leaf_hash(args.checkpoint_id, args.user_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_user_tree_merkle_proof(args.checkpoint_id, args.user_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserSubTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_user_sub_tree_merkle_proof(args.checkpoint_id, args.root_level, args.leaf_level, args.leaf_index)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetContractFunctionTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_contract_function_tree_root(args.checkpoint_id, args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetContractFunctionTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_contract_function_tree_leaf_hash(args.checkpoint_id, args.contract_id, args.function_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetContractFunctionTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_contract_function_tree_merkle_proof(args.checkpoint_id, args.contract_id, args.function_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetContractTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_contract_tree_root(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetContractTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_contract_tree_leaf_hash(args.checkpoint_id, args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetContractTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_contract_tree_merkle_proof(args.checkpoint_id, args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetDepositTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_deposit_tree_root(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetDepositTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_deposit_tree_leaf_hash(args.checkpoint_id, args.deposit_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetDepositTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_deposit_tree_merkle_proof(args.checkpoint_id, args.deposit_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetWithdrawalTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_withdrawal_tree_root(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetWithdrawalTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_withdrawal_tree_leaf_hash(args.checkpoint_id, args.withdrawal_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetWithdrawalTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_withdrawal_tree_merkle_proof(args.checkpoint_id, args.withdrawal_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetLatestCheckpointTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_latest_checkpoint_tree_root()?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetCheckpointTreeRoot(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_checkpoint_tree_root(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetCheckpointTreeLeafHash(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_checkpoint_tree_leaf_hash(args.checkpoint_id, args.leaf_checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetCheckpointTreeMerkleProof(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_checkpoint_tree_merkle_proof(args.checkpoint_id, args.leaf_checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }

        // Metadata commands
        Commands::GetContractLeafData(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let data = provider.get_contract_leaf_data(args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Commands::GetCheckpointLeafData(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let data = provider.get_checkpoint_leaf_data(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Commands::GetContractCodeDefinition(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let definition = provider.get_contract_code_definition(args.contract_id)?;
            println!("{}", serde_json::to_string_pretty(&definition)?);
        }
        Commands::GetLatestL2BlockState(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let state = provider.get_latest_l2_block_state()?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Commands::GetL2BlockState(args) => {
            use qed_prover::local::provider::RpcProvider;
            use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let state = provider.get_l2_block_state(args.checkpoint_id)?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }

        // wallet session
        Commands::WalletSession(wallet_session_args) => submit_end_cap_proof::run_multi(wallet_session_args)?,
        Commands::LocalProver(prover_args) => qed_prover::run_server(prover_args).await?,
        Commands::ProveProxy(prove_proxy_args) => {
            crate::subcommand::prove_proxy::run(prove_proxy_args).await?
        }
    }
    Ok(())
}
