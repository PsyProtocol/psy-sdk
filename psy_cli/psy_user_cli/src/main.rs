// #![cfg(feature = "is_sync")]
mod error;
mod subcommand;

use psy_prover::{local::native::prove_proxy, session};
#[cfg(not(target_arch = "wasm32"))]
use shadow_rs::shadow;

#[cfg(not(target_arch = "wasm32"))]
shadow!(build);

use clap::Parser;
use error::Result;

use crate::subcommand::{
    check_tx, claim_amount, claim_rewards, deploy_contract, register_user, submit_end_cap_proof, wallet, Cli, Commands,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    psy_common::setup_logging()?;
    tracing::info!("psy user cli");
    match cli.command {
        Commands::Wallet(args) => wallet::run(args)?,
        Commands::RegisterUser(args) => register_user::run(args).await?,
        Commands::DeployContract(args) => deploy_contract::run(args).await?,
        Commands::Call(args) => submit_end_cap_proof::run(args).await?,
        Commands::ClaimRewards(args) => claim_rewards::run(args).await?,
        Commands::GetClaimAmount(args) => claim_amount::run(args).await?,
        Commands::CheckTx(args) => check_tx::run(args).await?,

        // get block data
        Commands::GetUserId(user_id_args) => {
            use psy_rust_sdk::provider::RpcProvider;

            use crate::subcommand::args::UserIdArgs;
            let provider = RpcProvider::new_with_config_path(&user_id_args.rpc_config)?;
            let user_id = provider.get_user_id(user_id_args.pub_key).await?;
            println!("user_id: {}", user_id);
        }
        Commands::GetUserLeaf(user_leaf_args) => {
            use psy_crypto::hash::traits::qhashable::QFieldHashable;
            use psy_data::{config::store_config::PsyHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync};
            use psy_rust_sdk::provider::RpcProvider;

            use crate::subcommand::args::UserLeafArgs;

            let provider = RpcProvider::new_with_config_path(&user_leaf_args.rpc_config)?;

            let (user_id, query_method) = match (&user_leaf_args.pub_key, &user_leaf_args.user_id) {
                (Some(pub_key), None) => {
                    // Query by public key - get user_id from coordinator first
                    let user_id = provider.get_user_id(*pub_key).await?;
                    (user_id, "public_key")
                }
                (None, Some(user_id)) => {
                    // Query by user_id directly - use provided user_id
                    (*user_id, "user_id")
                }
                (Some(_), Some(_)) => {
                    return Err(anyhow::format_err!("Cannot specify both --pub-key and --user-id"));
                }
                (None, None) => {
                    return Err(anyhow::format_err!("Must specify either --pub-key or --user-id"));
                }
            };

            let user_leaf_data = provider.get_user_leaf_data(user_leaf_args.checkpoint_id, user_id).await?;
            println!("Query method: {}", query_method);
            println!("Resolved user_id: {}", user_id);
            println!("user_leaf_data: {}", serde_json::to_string_pretty(&user_leaf_data)?);
            println!("user_leaf_hash: {}", user_leaf_data.qfhash::<PsyHasher>().to_string());
        }

        // Tree commands
        Commands::GetUserContractStateTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider
                .get_user_contract_state_tree_root(args.checkpoint_id, args.user_id, args.contract_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserContractStateTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider
                .get_user_contract_state_tree_leaf_hash(args.checkpoint_id, args.user_id, args.contract_id, args.height, args.leaf_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserContractStateTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider
                .get_user_contract_state_tree_merkle_proof(args.checkpoint_id, args.user_id, args.contract_id, args.height, args.leaf_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserContractTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_user_contract_tree_root(args.checkpoint_id, args.user_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserContractTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider
                .get_user_contract_tree_leaf_hash(args.checkpoint_id, args.user_id, args.contract_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserContractTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider
                .get_user_contract_tree_merkle_proof(args.checkpoint_id, args.user_id, args.contract_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserRegistrationTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_user_registration_tree_root(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserRegistrationTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_user_registration_tree_leaf_hash(args.checkpoint_id, args.leaf_index).await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserRegistrationTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider
                .get_user_registration_tree_merkle_proof(args.checkpoint_id, args.leaf_index)
                .await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_user_tree_root(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetUserTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_user_tree_leaf_hash(args.checkpoint_id, args.user_id).await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetUserTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_user_tree_merkle_proof(args.checkpoint_id, args.user_id).await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetUserSubTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider
                .get_user_sub_tree_merkle_proof(args.checkpoint_id, args.root_level, args.leaf_level, args.leaf_index)
                .await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetContractFunctionTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_contract_function_tree_root(args.checkpoint_id, args.contract_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetContractFunctionTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider
                .get_contract_function_tree_leaf_hash(args.checkpoint_id, args.contract_id, args.function_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetContractFunctionTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider
                .get_contract_function_tree_merkle_proof(args.checkpoint_id, args.contract_id, args.function_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetContractTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_contract_tree_root(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetContractTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_contract_tree_leaf_hash(args.checkpoint_id, args.contract_id).await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetContractTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_contract_tree_merkle_proof(args.checkpoint_id, args.contract_id).await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetDepositTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_deposit_tree_root(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetDepositTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_deposit_tree_leaf_hash(args.checkpoint_id, args.deposit_id).await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetDepositTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_deposit_tree_merkle_proof(args.checkpoint_id, args.deposit_id).await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetWithdrawalTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_withdrawal_tree_root(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetWithdrawalTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider.get_withdrawal_tree_leaf_hash(args.checkpoint_id, args.withdrawal_id).await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetWithdrawalTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider.get_withdrawal_tree_merkle_proof(args.checkpoint_id, args.withdrawal_id).await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        Commands::GetLatestCheckpointTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_latest_checkpoint_tree_root().await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetCheckpointTreeRoot(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let root = provider.get_checkpoint_tree_root(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&root)?);
        }
        Commands::GetCheckpointTreeLeafHash(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let hash = provider
                .get_checkpoint_tree_leaf_hash(args.checkpoint_id, args.leaf_checkpoint_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&hash)?);
        }
        Commands::GetCheckpointTreeMerkleProof(args) => {
            use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let proof = provider
                .get_checkpoint_tree_merkle_proof(args.checkpoint_id, args.leaf_checkpoint_id)
                .await?;
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }

        // Metadata commands
        Commands::GetContractLeafData(args) => {
            use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let data = provider.get_contract_leaf_data(args.contract_id).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Commands::GetCheckpointLeafData(args) => {
            use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let data = provider.get_checkpoint_leaf_data(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Commands::GetContractCodeDefinition(args) => {
            use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let definition = provider.get_contract_code_definition(args.contract_id).await?;
            println!("{}", serde_json::to_string_pretty(&definition)?);
        }
        Commands::GetLatestBlockState(args) => {
            use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let state = provider.get_latest_block_state().await?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Commands::GetBlockState(args) => {
            use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
            use psy_rust_sdk::provider::RpcProvider;
            let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
            let state = provider.get_block_state(args.checkpoint_id).await?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }

        // wallet session
        Commands::LocalProver(prover_args) => psy_prover::run_server(prover_args).await?,
        Commands::ProveProxy(prove_proxy_args) => crate::subcommand::prove_proxy::run(prove_proxy_args).await?,
    }
    Ok(())
}
