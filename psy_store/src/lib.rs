#[cfg(not(target_arch = "wasm32"))]
pub mod node;
#[cfg(not(target_arch = "wasm32"))]
pub mod queue;
#[cfg(not(target_arch = "wasm32"))]
pub mod store;

use anyhow::Result;
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use plonky2::field::types::Field;
use psy_core::config::network_constants::GLOBAL_USER_TREE_HEIGHT;
#[cfg(not(target_arch = "wasm32"))]
use psy_data::qblock::process::simple::SimpleBlockProcessor;
use psy_data::{
    config::store_config::PsyFelt,
    qblock::cmds::{deploy_contract::QBCDeployContract, register_user::QBCRegisterUser},
    traits::qdatastore::qmetadata::{QMetaDataStoreReaderSync, QMetaDataStoreWriterSync},
};

#[cfg(all(not(target_arch = "wasm32")))]
pub async fn prepare_environment_with_real_contract(
    register_users: Vec<QBCRegisterUser<PsyFelt>>,
    deploy_contracts: Vec<QBCDeployContract<PsyFelt>>,
    user_id: Option<u64>,
    nonce: Option<PsyFelt>,
    session_proof_tree_height: Option<usize>,
) -> Result<psy_data::qstore::controllers::proving_session::PsyLocalProvingSessionStore<PsyFelt, KVQSimpleMemoryBackingStore>> {
    use crate::node::coordinator::PsyCoordinatorStoreWriterAsyncImm;

    let store = KVQSimpleMemoryBackingStore::new();
    store.initialize_store(None).await?;

    let final_store = SimpleBlockProcessor::prepare_environment_with_real_contract(register_users, deploy_contracts, store).await?;

    let latest_block_state = final_store.get_latest_block_state().await?;
    let final_user_id = PsyFelt::from_canonical_u64(user_id.unwrap_or(5));
    let final_nonce = nonce.unwrap_or(PsyFelt::ZERO);
    let final_height = session_proof_tree_height.unwrap_or(GLOBAL_USER_TREE_HEIGHT as usize);
    let final_checkpoint_id = PsyFelt::from_canonical_u64(latest_block_state.checkpoint_id);

    Ok(psy_data::qstore::controllers::proving_session::PsyLocalProvingSessionStore::new_at(
        final_store,
        final_checkpoint_id,
        final_user_id,
        final_nonce,
        final_height,
    ))
}
