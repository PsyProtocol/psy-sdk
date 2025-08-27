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
use qed_data::qdata::checkpoint::QEDL2BlockState;

#[cfg(all(not(target_arch = "wasm32"), feature = "is_sync"))]
pub async fn prepare_environment_with_real_contract(
    new_user_public_key: QBCRegisterUser<QEDFelt>,
    deploy_contract: QBCDeployContract<QEDFelt>,
) -> Result<QEDLocalProvingSessionStore<QEDFelt, KVQSimpleMemoryBackingStore>> {
    use crate::node::coordinator::QEDCoordinatorStoreWriterAsyncImm;

    let store = KVQSimpleMemoryBackingStore::new();

    store.initialize_store(None).await?;

    let store = SimpleBlockProcessor::prepare_environment_with_real_contract(
        new_user_public_key,
        deploy_contract,
        store,
    )?;

    let latest_l2_block_state = store.get_latest_l2_block_state()?;
    let user_id = QEDFelt::from_canonical_u64(5);
    let nonce = QEDFelt::ZERO;

    Ok(QEDLocalProvingSessionStore::new_at(
        store,
        QEDFelt::from_canonical_u64(latest_l2_block_state.checkpoint_id),
        user_id,
        nonce,
        GLOBAL_USER_TREE_HEIGHT as usize,
    ))
}
