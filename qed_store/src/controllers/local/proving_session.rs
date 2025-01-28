use kvq::memory::{arc_imm::KVQArcImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore};
use plonky2::{field::{goldilocks_field::GoldilocksField, packed::PackedField, types::{Field, PrimeField64}}, hash::hash_types::RichField};
use qed_core::{config::network_constants::DEFERRED_TRANSACTION_TREE_HEIGHT, data::qhashout::QHashOut};
use qed_crypto::hash::{merkle::core::{DeltaMerkleProofCore, MerkleProofCore}, utils::safe_hash_fixed_length};
use qed_data::{dpn::{cfc_context_input::{DapenCFCProvingSessionStartContext, DapenCFCUserTransactionCallStartContext}, proving_session::{DPNProvingSessionCompactMethodCall, DPNProvingSessionSimpleMethodCall}}, qdata::checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf}};
use serde::{Deserialize, Serialize};

use crate::{config::store_config::{QEDDeltaMerkleProof, QEDHasher, UserContractTreeStore}, models::{kvq_merkle::model::{KVQMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelReaderCore}, user::contract_state_tree::UserContractStateTreeId}, store::imm::{cache::QEDCmdStoreWithCache, cmd::{QSRCmdGetCheckpointLeafData, QSRCmdGetContractLeafData, QSRCmdGetUserLeafData, QSRHashCmd, QSRHashCmdGetCheckpointTreeRoot, QSRHashCmdGetContractTreeRoot, QSRHashCmdGetDepositTreeRoot, QSRHashCmdGetUserTreeRoot, QSRHashCmdGetWithdrawalTreeRoot, QSRMerkleCmd, QSRMerkleCmdGetCheckpointTreeMerkleProof, QSRMerkleCmdGetUserContractStateTreeMerkleProof, QSRMerkleCmdGetUserContractTreeMerkleProof, QSRMerkleCmdGetUserTreeMerkleProof}, cmd_processor::{DPNReadOtherUserLeafMerkleProof, QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}}, traits::qdatastore::qtreedata::QEDComboDataStoreReaderSync};

use super::session_store::{config::LPS_DEFERRED_TRANSACTION_TREE_ID, tx_tree::TransactionDebtTreeRef};

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDLocalStateSet<F: RichField> {
    pub contract: F,
    pub slot: F,
    pub contract_state_transition_proof: DeltaMerkleProofCore<QHashOut<F>>,
}


#[derive(Clone, Debug)]
pub struct QEDLocalProvingSessionStore<F: RichField, R: QEDReadCommandProcessorSync<F>> {
    pub cmd_store: QEDCmdStoreWithCache<F, R>,
    pub state_tree_store: KVQSimpleMemoryBackingStore,
    pub active_tx_session_data_store: KVQSimpleMemoryBackingStore,

    pub deferred_tx_debt_store: TransactionDebtTreeRef<DPNProvingSessionSimpleMethodCall<F>, F, DEFERRED_TRANSACTION_TREE_HEIGHT, LPS_DEFERRED_TRANSACTION_TREE_ID>,

    //pub delta_merkle_proof_cache: Vec<QEDLocalStateSet<F>>,


    pub start_checkpoint: F,
    pub write_checkpoint: F,
    pub user_id: F,
    pub current_contract_id: F,

    pub start_checkpoint_u64: u64,
    pub write_checkpoint_u64: u64,
    pub user_id_u64: u64,
    pub user_id_u32: u32,
    pub nonce: F,
}
impl<F: RichField, R: QEDReadCommandProcessorSync<F>> QEDLocalProvingSessionStore<F, R> {
    pub fn new_at(read_store: R, start_checkpoint: F, user_id: F, contract_id: F, nonce: F) -> Self {
        Self {
            cmd_store: QEDCmdStoreWithCache::new(start_checkpoint.to_canonical_u64(), read_store),
            state_tree_store:  KVQSimpleMemoryBackingStore::new(),
            active_tx_session_data_store:  KVQSimpleMemoryBackingStore::new(),
            deferred_tx_debt_store: TransactionDebtTreeRef::new(start_checkpoint.to_canonical_u64()),
            //delta_merkle_proof_cache: Vec::new(),
            start_checkpoint: start_checkpoint,
            write_checkpoint: start_checkpoint+F::ONE,
            current_contract_id: contract_id,
            user_id,
            start_checkpoint_u64: start_checkpoint.to_canonical_u64(),
            write_checkpoint_u64: start_checkpoint.to_canonical_u64()+1,
            user_id_u64: user_id.to_canonical_u64(),
            user_id_u32: user_id.to_canonical_u64() as u32,
            nonce,
        }
    }
    pub fn new_at_head(read_store: R, user_id: F, contract_id: F, nonce: F) -> anyhow::Result<Self> {
        let start_checkpoint = read_store.resolve_get_latest_l2_block_state()?;

        Ok(Self::new_at(read_store, F::from_noncanonical_u64(start_checkpoint.checkpoint_id), user_id, contract_id, nonce))
    }
    pub fn clear(&mut self) {
        self.cmd_store.clear_cache_mut();
        self.state_tree_store.clear();
        self.write_checkpoint = self.start_checkpoint + F::ONE;
        self.write_checkpoint_u64 = self.start_checkpoint_u64 + 1;
    }
}

type GF = GoldilocksField;
impl<R: QEDReadCommandProcessorSync<GoldilocksField>> QEDLocalProvingSessionStore<GoldilocksField, R> {
    /*
    pub fn get_full_data(&mut self, contract_id: GF, method_id: GF, inputs: &[GF], outputs: &[GF]) -> anyhow::Result<DapenCFCUserTransactionCallStartContext<GF>> {
    }*/

    pub fn get_call_start_data(&mut self, contract_id: GF, method_id: GF, inputs: &[GF]) -> anyhow::Result<DapenCFCUserTransactionCallStartContext<GF>> {

        let contract_state_root_proof = self.get_self_user_contract_tree_leaf(contract_id)?;

        
        let start_user_contract_tree_root  = contract_state_root_proof.root;
        let start_contract_state_tree_root = contract_state_root_proof.value;

        let call_data = DPNProvingSessionCompactMethodCall{
            contract_id,
            method_id,
            inputs_length: GF::from_canonical_u64(inputs.len() as u64),
            inputs_hash: safe_hash_fixed_length::<QEDHasher, GF>(inputs),
        };

        let start_deferred_tx_debt_tree_root = self.get_latest_deferred_tx_leaf()?.root;
        let start_user_balance = GF::ZERO;
        let start_user_event_index = GF::ZERO;

        Ok(DapenCFCUserTransactionCallStartContext{
            start_user_contract_tree_root,
            start_contract_state_tree_root,
            call_data,
            start_deferred_tx_debt_tree_root,
            start_user_balance,
            start_user_event_index,
        })
        
        
    }
    pub fn get_global_state_tree_roots(&mut self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<GF>> {
        let contract_tree_root = self.cmd_store.resolve_get_hash_mut(&QSRHashCmd::GetContractTreeRoot(QSRHashCmdGetContractTreeRoot{
            checkpoint_id,
        }))?;

        let deposit_tree_root = self.cmd_store.resolve_get_hash_mut(&QSRHashCmd::GetDepositTreeRoot(QSRHashCmdGetDepositTreeRoot{
            checkpoint_id,
        }))?;
        let user_tree_root = self.cmd_store.resolve_get_hash_mut(&QSRHashCmd::GetUserTreeRoot(QSRHashCmdGetUserTreeRoot{
            checkpoint_id,
        }))?;
        let withdrawal_tree_root = self.cmd_store.resolve_get_hash_mut(&QSRHashCmd::GetWithdrawalTreeRoot(QSRHashCmdGetWithdrawalTreeRoot{
            checkpoint_id,
        }))?;

        Ok(
            QEDCheckpointGlobalStateRoots {
                contract_tree_root,
                deposit_tree_root,
                user_tree_root,
                withdrawal_tree_root,
            }
        )

    }
    pub fn get_fresh_start_ctx_for_user(&mut self, user: GF) -> anyhow::Result<DapenCFCProvingSessionStartContext<GF>> {

        let checkpoint_id = self.start_checkpoint_u64;
        let checkpoint_leaf = self.cmd_store.resolve_get_checkpoint_leaf_mut(&QSRCmdGetCheckpointLeafData{
            checkpoint_id,
        })?;
        let checkpoint_tree_root = self.cmd_store.resolve_get_hash_mut(&QSRHashCmd::GetCheckpointTreeRoot(QSRHashCmdGetCheckpointTreeRoot{
            checkpoint_id,
        }))?;
        let state_roots = self.get_global_state_tree_roots(checkpoint_id)?;


        let user_leaf = self.cmd_store.resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData{
            checkpoint_id: self.start_checkpoint_u64+1000,
            user_id: user.to_canonical_u64(),
        })?;

        if user_leaf.last_checkpoint_id.to_canonical_u64() > checkpoint_id {
            anyhow::bail!("the user's checkpoint is ahead of the proving session (user sync'd to {}, proving session on checkpoint {})", user_leaf.last_checkpoint_id.to_canonical_u64(), checkpoint_id);
        }

        let res = DapenCFCProvingSessionStartContext {
            checkpoint_id: GF::from_canonical_u64(checkpoint_id),
            checkpoint_tree_root,
            checkpoint_leaf,
            state_roots,
            start_session_user_leaf: user_leaf,
        };

        Ok(res)
        
    }
    pub fn set_contract_state_slot(&mut self, contract: GF, slot: GF, value: QHashOut<GF>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let state_tree_height = self.cmd_store.resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData{contract_id: contract.to_canonical_u64()})?.state_tree_height.to_canonical_u64() as u8;
        let id = UserContractStateTreeId::new(self.user_id_u64, contract.to_canonical_u64() as u32, state_tree_height);
        let base_mp = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(QSRMerkleCmdGetUserContractStateTreeMerkleProof{
            checkpoint_id: self.start_checkpoint_u64,
            user_id: self.user_id_u64,
            contract_id: contract.to_canonical_u64() as u32,
            height: state_tree_height,
            leaf_id: slot.to_canonical_u64(),
        }))?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, self.start_checkpoint_u64, &base_mp)?;
        let dmp = id.set_leaf_ucs(&mut self.state_tree_store, self.write_checkpoint_u64, slot.to_canonical_u64(), value)?;
        /*let cache_value = QEDLocalStateSet{
            contract,
            slot,
            contract_state_transition_proof: dmp.clone(),
        };
        self.delta_merkle_proof_cache.push(cache_value);*/

        Ok(dmp)
       // self.state_tree_store.map.insert((self.user_id_u64, contract, slot), value);
    }
    pub fn get_contract_state_slot(&mut self, contract: GF, slot: GF) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        let state_tree_height = self.cmd_store.resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData{contract_id: contract.to_canonical_u64()})?.state_tree_height.to_canonical_u64() as u8;
        let id = UserContractStateTreeId::new(self.user_id_u64, contract.to_canonical_u64() as u32, state_tree_height);
        let base_mp = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(QSRMerkleCmdGetUserContractStateTreeMerkleProof{
            checkpoint_id: self.start_checkpoint_u64,
            user_id: self.user_id_u64,
            contract_id: contract.to_canonical_u64() as u32,
            height: state_tree_height,
            leaf_id: slot.to_canonical_u64(),
        }))?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, self.start_checkpoint_u64, &base_mp)?;
        id.get_leaf_ucs(&self.state_tree_store, self.write_checkpoint_u64, slot.to_canonical_u64())
       // self.state_tree_store.map.insert((self.user_id_u64, contract, slot), value);
    }
    pub fn get_self_user_contract_tree_leaf(&mut self, contract_id: GF) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {

        let old_upper_merkle_proof = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(QSRMerkleCmdGetUserContractTreeMerkleProof{
            checkpoint_id: self.start_checkpoint_u64,
            user_id: self.user_id_u64,
            contract_id: contract_id.to_canonical_u64() as u32,
        }))?;
        
        UserContractTreeStore::injest_merkle_proof_sfc(&mut self.state_tree_store, self.user_id_u64, self.start_checkpoint_u64, &old_upper_merkle_proof)?;
        UserContractTreeStore::get_leaf_sfc(&self.state_tree_store, self.write_checkpoint_u64, self.user_id_u64, contract_id.to_canonical_u64())
    }

    pub fn update_contract_state_root_in_user_contract_tree(&mut self, contract_id: GF) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let latest_root = self.get_contract_state_slot(contract_id, GF::ZERO)?.root;

        let old_upper_merkle_proof = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(QSRMerkleCmdGetUserContractTreeMerkleProof{
            checkpoint_id: self.start_checkpoint_u64,
            user_id: self.user_id_u64,
            contract_id: contract_id.to_canonical_u64() as u32,
        }))?;
        
        UserContractTreeStore::injest_merkle_proof_sfc(&mut self.state_tree_store, self.user_id_u64, self.start_checkpoint_u64, &old_upper_merkle_proof)?;
        UserContractTreeStore::set_leaf_sfc(&mut self.state_tree_store, self.write_checkpoint_u64, self.user_id_u64, contract_id.to_canonical_u64(), latest_root)
    }
    pub fn get_external_user_leaf_proof(&mut self, user_id: GF) -> anyhow::Result<DPNReadOtherUserLeafMerkleProof<GF>> {
        let user_tree_proof = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserTreeMerkleProof(QSRMerkleCmdGetUserTreeMerkleProof{checkpoint_id: self.start_checkpoint_u64, user_id: user_id.to_canonical_u64()}))?;
        let user_leaf = self.cmd_store.resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData{checkpoint_id: self.start_checkpoint_u64, user_id: user_id.to_canonical_u64()})?;
        Ok(
            DPNReadOtherUserLeafMerkleProof{
                user_tree_proof,
                user_leaf,
            }
        )

    }
    pub fn add_deferred_tx_to_debt(&mut self, tx: DPNProvingSessionSimpleMethodCall<GF>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        self.deferred_tx_debt_store.add_tx_debt(&mut self.active_tx_session_data_store, tx)
    }
    pub fn get_deferred_tx_leaf(&self, leaf_index: GF) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>>{
        self.deferred_tx_debt_store.get_tx_debt_leaf(&self.active_tx_session_data_store, leaf_index.to_canonical_u64())
    }
    pub fn get_latest_deferred_tx_leaf(&self) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>>{
        self.deferred_tx_debt_store.get_latest_tx_debt_leaf(&self.active_tx_session_data_store)
    }
}

