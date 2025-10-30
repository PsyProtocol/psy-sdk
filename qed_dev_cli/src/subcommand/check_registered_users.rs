use clap::Parser;
use qed_core::config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, GROUP_REALM_HEIGHT, REALM_USER_TREE_HEIGHT};
use qed_crypto::common::user_id::{
    self, UserIdBitsStrategy1, UserIdBitsStrategy2, UserIdBitsStrategy3, UserIdBitsStrategy4, UserIdGeneratorStrategy,
};
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_prover::local::provider::RpcProvider;


#[derive(Parser)]
pub struct CheckRegisteredUsersArgs {
    #[clap(env, long, default_value = "config.json", env)]
    pub rpc_config: String,
    #[clap(env, long, env)]
    pub start_registration_id: Option<u64>,
    #[clap(env, long, env)]
    pub end_registration_id: Option<u64>,
}

pub async fn run(args: CheckRegisteredUsersArgs) -> anyhow::Result<()> {
    let mut provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    let mut error_registered_user_ids = Vec::new();
    let mut error_registered_ids = Vec::new();

    let latest_l2_block_state = provider.get_latest_l2_block_state().await?;
    let start_registration_id = args.start_registration_id.unwrap_or_default();
    let end_registration_id = args.end_registration_id.unwrap_or(latest_l2_block_state.next_user_id);

    for registration_id in start_registration_id..end_registration_id {
        tracing::info!("check registration_id {}", registration_id);
        let user_id = UserIdBitsStrategy4::get_user_id_from_registration_id(registration_id);

        match provider.get_user_leaf_data(latest_l2_block_state.checkpoint_id + 1000, user_id).await {
            Ok(user_leaf) => tracing::info!("Register {} User {}: {}", registration_id, user_id, user_leaf.public_key),
            Err(e) => {
                tracing::warn!("Error `{}` Register {} User {}: Not registered", e.to_string(), registration_id, user_id);
                error_registered_user_ids.push(user_id);
                error_registered_ids.push(registration_id);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    tracing::info!("Error registered ids: {:?}", error_registered_ids);
    tracing::info!("Error registered user ids: {:?}", error_registered_user_ids);
    Ok(())
}
