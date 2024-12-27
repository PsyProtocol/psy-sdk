use kvq::memory::{arc_imm::KVQArcImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore};
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use serde::{Deserialize, Serialize};

use crate::{store::imm::{cache::QEDCmdStoreWithCache, cmd_processor::{QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}}, traits::qdatastore::qtreedata::QEDComboDataStoreReaderSync};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDLocalTransactionCallInfo<F: RichField> {
    pub start_checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
    pub function_id: F,
    pub caller_contract_id: F,
    pub flat_args: Vec<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDLocalTransactionStateHints<F: RichField> {
    pub call_info: QEDLocalTransactionCallInfo<F>,
    pub start_contract_state_tree_root: QHashOut<F>,
    pub contract_state_tree_height: u16,

    pub start_checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
    pub function_id: F,
    pub caller_contract_id: F,
    pub flat_args: Vec<F>,
}


#[derive(Clone, Debug)]
pub struct QEDLocalProvingSessionStore<F: RichField, R: QEDReadCommandProcessorSync<F>> {
    pub cmd_store: QEDCmdStoreWithCache<F, R>,
    pub state_tree_store: KVQSimpleMemoryBackingStore,


    pub start_checkpoint: F,
    pub write_checkpoint: F,
    pub user_id: F,

    pub start_checkpoint_u64: u64,
    pub write_checkpoint_u64: u64,
    pub user_id_u64: u64,
    pub user_id_u32: u32,
}
impl<F: RichField, R: QEDReadCommandProcessorSync<F>> QEDLocalProvingSessionStore<F, R> {
    pub fn new_at(read_store: R, start_checkpoint: F, user_id: F) -> Self {
        Self {
            cmd_store: QEDCmdStoreWithCache::new(start_checkpoint.to_canonical_u64(), read_store),
            state_tree_store:  KVQSimpleMemoryBackingStore::new(),
            start_checkpoint: start_checkpoint,
            write_checkpoint: start_checkpoint+F::ONE,
            user_id,
            start_checkpoint_u64: start_checkpoint.to_canonical_u64(),
            write_checkpoint_u64: start_checkpoint.to_canonical_u64()+1,
            user_id_u64: user_id.to_canonical_u64(),
            user_id_u32: user_id.to_canonical_u64() as u32,
        }
    }
    pub fn new_at_head(read_store: R, user_id: F) -> anyhow::Result<Self> {
        let start_checkpoint = read_store.resolve_get_latest_l2_block_state()?;

        Ok(Self::new_at(read_store, F::from_noncanonical_u64(start_checkpoint.checkpoint_id), user_id))
    }
    pub fn clear(&mut self) {
        self.cmd_store.clear_cache_mut();
        self.state_tree_store.clear();
        self.write_checkpoint = self.start_checkpoint + F::ONE;
        self.write_checkpoint_u64 = self.start_checkpoint_u64 + 1;
    }
    pub fn set_contract_state_slot(contract: F, slot: F, value: QHashOut<F>) {
        todo!()
       // self.state_tree_store.map.insert((self.user_id_u64, contract, slot), value);
    }
}

