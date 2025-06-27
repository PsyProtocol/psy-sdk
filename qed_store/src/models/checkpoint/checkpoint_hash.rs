use kvq::traits::{
    KVQBinaryStore, KVQPair, KVQStoreAdapter, KVQStoreAdapterReader
};
use qed_data::qdata::{hash_cache_result::QEDHashHelperResult, hash_key::Hash4x64Key};

use crate::
    config::store_config::QEDHash
;

pub trait QEDCheckpointHashHelperModelReaderCore<
    const CHECKPOINT_HASH_HELPER_TABLE_TYPE: u16,
    S: KVQBinaryStore,
    KVA: KVQStoreAdapterReader<
        S,
        Hash4x64Key<CHECKPOINT_HASH_HELPER_TABLE_TYPE>,
        QEDHashHelperResult,
    >,
>
{
    fn get_checkpoint_hash_helper_info(
        store: &S,
        hash: QEDHash,
    ) -> anyhow::Result<QEDHashHelperResult> {
        //tracing::info!("get block state: {}", checkpoint_id);
        KVA::get_exact(store, &hash.into())
    }
    fn get_checkpoint_hash_helper_info_if_exists(
        store: &S,
        hash: QEDHash,
    ) -> anyhow::Result<Option<QEDHashHelperResult>> {
        //tracing::info!("get block state: {}", checkpoint_id);
        KVA::get_exact_if_exists(store, &hash.into())
    }
}
pub trait QEDCheckpointHashHelperModelCore<
    const CHECKPOINT_HASH_HELPER_TABLE_TYPE: u16,
    S: KVQBinaryStore,
    KVA: KVQStoreAdapter<
        S,
        Hash4x64Key<CHECKPOINT_HASH_HELPER_TABLE_TYPE>,
        QEDHashHelperResult,
    >,
>: QEDCheckpointHashHelperModelReaderCore<CHECKPOINT_HASH_HELPER_TABLE_TYPE, S, KVA>
{
    fn delete_checkpoint_hash_helper_info(
        store: &mut S,
        hash: QEDHash,
    ) -> anyhow::Result<Option<QEDHashHelperResult>> {
        let current = KVA::get_exact_if_exists(store, &hash.into())?;
        if current.is_some() {
            let deposit = current.unwrap();
            KVA::delete(store, &hash.into())?;
            Ok(Some(deposit))
        } else {
            Ok(None)
        }
    }
    fn set_checkpoint_hash_helper_info(
        store: &S,
        checkpoint_id: u64,
        checkpoint_leaf_hash: QEDHash,
        checkpoint_tree_root_hash: QEDHash,
    ) -> anyhow::Result<()> {
        KVA::set_many(
            store,
            &[
                KVQPair{
                    key: checkpoint_leaf_hash.into(),
                    value: QEDHashHelperResult::new_checkpoint_leaf_hash(checkpoint_id),
                },
                KVQPair{
                    key: checkpoint_tree_root_hash.into(),
                    value: QEDHashHelperResult::new_checkpoint_tree_root_hash(checkpoint_id),
                }
            ]
        )?;
        Ok(())
    }
}
pub struct QEDCheckpointHashHelperModel<const CHECKPOINT_HASH_HELPER_TABLE_TYPE: u16, S, KVA> {
    _store: S,
    _kva: KVA,
}

impl<
        const CHECKPOINT_HASH_HELPER_TABLE_TYPE: u16,
        S: KVQBinaryStore,
        KVA: KVQStoreAdapterReader<
            S,
            Hash4x64Key<CHECKPOINT_HASH_HELPER_TABLE_TYPE>,
            QEDHashHelperResult,
        >,
    > QEDCheckpointHashHelperModelReaderCore<CHECKPOINT_HASH_HELPER_TABLE_TYPE, S, KVA>
    for QEDCheckpointHashHelperModel<CHECKPOINT_HASH_HELPER_TABLE_TYPE, S, KVA>
{
}
impl<
        const CHECKPOINT_HASH_HELPER_TABLE_TYPE: u16,
        S: KVQBinaryStore,
        KVA: KVQStoreAdapter<
            S,
            Hash4x64Key<CHECKPOINT_HASH_HELPER_TABLE_TYPE>,
            QEDHashHelperResult,
        >,
    > QEDCheckpointHashHelperModelCore<CHECKPOINT_HASH_HELPER_TABLE_TYPE, S, KVA>
    for QEDCheckpointHashHelperModel<CHECKPOINT_HASH_HELPER_TABLE_TYPE, S, KVA>
{
}
