use kvq::traits::{KVQBinaryStoreImmutable, KVQBinaryStoreReader, KVQStoreAdapterImmutable, KVQStoreAdapterReader};
use qed_data::qdata::{checkpoint::QEDL2BlockState, u64_key::U64TableKey};

use crate::models::kvq_merkle::model::CHECKPOINT_ID_FUZZY_SIZE;


pub trait L2BlockStatesModelReaderCore<
    const CHECKPOINT_BLOCK_STATE_TABLE_TYPE: u16,
    S: KVQBinaryStoreReader,
    KVA: KVQStoreAdapterReader<S, U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>, QEDL2BlockState>,
>
{
    fn get_block_state_by_id(store: &S, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        //tracing::info!("get block state: {}", checkpoint_id);
        KVA::get_exact(store, &U64TableKey(checkpoint_id))
    }
    fn get_latest_block_state(store: &S) -> anyhow::Result<QEDL2BlockState> {
        let result = KVA::get_leq(
            store,
            &U64TableKey(0xffffffffffffffu64),
            CHECKPOINT_ID_FUZZY_SIZE,
        )?;
        if result.is_some() {
            Ok(result.unwrap())
        } else {
            anyhow::bail!("error getting latest block state")
        }
    }
    fn get_block_states_by_id(
        store: &S,
        checkpoint_ids: &[u64],
    ) -> anyhow::Result<Vec<QEDL2BlockState>> {
        let keys = checkpoint_ids
            .iter()
            .map(|id| U64TableKey(*id))
            .collect::<Vec<_>>();
        KVA::get_many_exact(store, &keys)
    }
}
pub trait L2BlockStatesModelCore<
    const CHECKPOINT_BLOCK_STATE_TABLE_TYPE: u16,
    S: KVQBinaryStoreImmutable,
    KVA: KVQStoreAdapterImmutable<S, U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>, QEDL2BlockState>,
>: L2BlockStatesModelReaderCore<CHECKPOINT_BLOCK_STATE_TABLE_TYPE, S, KVA>
{
    fn delete_block_state_by_id(
        store: &S,
        checkpoint_id: u64,
    ) -> anyhow::Result<Option<QEDL2BlockState>> {
        let key_id = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(checkpoint_id);
        let current = KVA::get_exact_if_exists(store, &key_id)?;
        if current.is_some() {
            let deposit = current.unwrap();
            KVA::imm_delete(store, &key_id)?;
            Ok(Some(deposit))
        } else {
            Ok(None)
        }
    }
    fn set_block_state(store: &S, block_state: QEDL2BlockState) -> anyhow::Result<()> {
        let key_id = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(block_state.checkpoint_id);
        KVA::imm_set(store, key_id, block_state)?;
        Ok(())
    }
    fn set_block_state_ref(store: &S, block_state: &QEDL2BlockState) -> anyhow::Result<()> {
        let key_id = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(block_state.checkpoint_id);
        KVA::imm_set_ref(store, &key_id, &block_state)?;
        Ok(())
    }
    fn set_block_states(store: &S, block_states: &[QEDL2BlockState]) -> anyhow::Result<()> {
        let key_ids = block_states
            .iter()
            .map(|s| U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(s.checkpoint_id))
            .collect::<Vec<_>>();
        KVA::imm_set_many_split_ref(store, &key_ids, block_states)?;

        Ok(())
    }
}
pub struct L2BlockStatesModel<const CHECKPOINT_BLOCK_STATE_TABLE_TYPE: u16, S, KVA> {
    _store: S,
    _kva: KVA,
}

impl<
        const CHECKPOINT_BLOCK_STATE_TABLE_TYPE: u16,
        S: KVQBinaryStoreReader,
        KVA: KVQStoreAdapterReader<S, U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>, QEDL2BlockState>,
    > L2BlockStatesModelReaderCore<CHECKPOINT_BLOCK_STATE_TABLE_TYPE, S, KVA>
    for L2BlockStatesModel<CHECKPOINT_BLOCK_STATE_TABLE_TYPE, S, KVA>
{
}
impl<
        const CHECKPOINT_BLOCK_STATE_TABLE_TYPE: u16,
        S: KVQBinaryStoreImmutable,
        KVA: KVQStoreAdapterImmutable<S, U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>, QEDL2BlockState>,
    > L2BlockStatesModelCore<CHECKPOINT_BLOCK_STATE_TABLE_TYPE, S, KVA>
    for L2BlockStatesModel<CHECKPOINT_BLOCK_STATE_TABLE_TYPE, S, KVA>
{
}