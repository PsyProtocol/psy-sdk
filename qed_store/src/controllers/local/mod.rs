pub mod proving_session;
pub mod session_store;
pub mod session_info;
pub mod state_tracker;

use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use qed_data::qblock::process::simple::SimpleBlockProcessor;
use qed_data::qblock::cmds::register_user::QBCRegisterUser;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qed_data::config::store_config::QEDFelt;
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use crate::controllers::local::proving_session::QEDLocalProvingSessionStore;
use qed_data::traits::qdatastore::qmetadata::{QMetaDataStoreReaderSync, QMetaDataStoreWriterSync};
use qed_core::config::network_constants::GLOBAL_USER_TREE_HEIGHT;
use plonky2::field::types::Field;

#[cfg(all(not(target_arch = "wasm32")))]
pub async fn prepare_environment_with_real_contract(
    register_users: Vec<QBCRegisterUser<QEDFelt>>,
    deploy_contracts: Vec<QBCDeployContract<QEDFelt>>,
    user_id: Option<u64>,
    nonce: Option<QEDFelt>,
    session_proof_tree_height: Option<usize>,
) -> Result<QEDLocalProvingSessionStore<QEDFelt, KVQSimpleMemoryBackingStore>> {
    use crate::node::coordinator::QEDCoordinatorStoreWriterAsyncImm;

    let store = KVQSimpleMemoryBackingStore::new();
    store.initialize_store(None).await?;

    let final_store = SimpleBlockProcessor::prepare_environment_with_real_contract(
        register_users,
        deploy_contracts,
        store,
    ).await?;

    let latest_l2_block_state = final_store.get_latest_l2_block_state().await?;
    let final_user_id = QEDFelt::from_canonical_u64(user_id.unwrap_or(5));
    let final_nonce = nonce.unwrap_or(QEDFelt::ZERO);
    let final_height = session_proof_tree_height.unwrap_or(GLOBAL_USER_TREE_HEIGHT as usize);
    let final_checkpoint_id = QEDFelt::from_canonical_u64(latest_l2_block_state.checkpoint_id);

    Ok(QEDLocalProvingSessionStore::new_at(
        final_store,
        final_checkpoint_id,
        final_user_id,
        final_nonce,
        final_height,
    ))
}
