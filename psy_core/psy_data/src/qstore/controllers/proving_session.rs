use hashbrown::HashMap;
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::hash_types::RichField,
};
use psy_common::data::qhashout::QHashOut;
use psy_config::network_constants::{DEFERRED_TRANSACTION_TREE_HEIGHT, INLINE_TRANSACTION_TREE_HEIGHT};
use psy_crypto::hash::{
    merkle::{
        core::{DeltaMerkleProofCore, MerkleProofCore},
        utils::simple_merkle_tree::SimpleMerkleTree,
    },
    traits::{hasher::MerkleZeroHasher, qhashable::QFieldHashable},
    utils::safe_hash_fixed_length,
};

use super::{
    session_store::{config::LPS_DEFERRED_TRANSACTION_TREE_ID, tx_tree::TransactionDebtTreeRef},
    state_tracker::PsyLocalStateTracker,
};
use crate::{
    config::store_config::{PsyHasher, UserContractTreeStore},
    dpn::{
        cfc_context_input::{DapenCFCProvingSessionStartContext, DapenCFCUserTransactionCallStartContext},
        proving_session::{
            DPNProvingSessionCompactMethodCall, DPNProvingSessionSignableMethodCall, DPNProvingSessionSimpleMethodCall, DPNTransactionDebtItem,
            PsyLocalTransactionRecord,
        },
    },
    guta::api::PsyContractStateUpdateHistory,
    models::{
        kvq_merkle::model::{KVQSemiFixedConfigMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelReaderCore},
        user::contract_state_tree::UserContractStateTreeId,
    },
    qdata::{
        checkpoint::PsyCheckpointGlobalStateRoots,
        contract_inclusion::{PsyContractFunctionInclusionProof, PsyContractInclusionProof},
    },
    qstore::imm::{
        cache::PsyCmdStoreWithCache,
        cmd::{
            QSRCmdGetCheckpointLeafData, QSRCmdGetContractLeafData, QSRCmdGetUserLeafData, QSRHashCmd, QSRHashCmdGetCheckpointTreeRoot,
            QSRHashCmdGetContractTreeRoot, QSRHashCmdGetDepositTreeRoot, QSRHashCmdGetUserRegistrationTreeRoot, QSRHashCmdGetUserTreeRoot,
            QSRHashCmdGetWithdrawalTreeRoot, QSRMerkleCmd, QSRMerkleCmdGetContractFunctionTreeMerkleProof, QSRMerkleCmdGetContractTreeMerkleProof,
            QSRMerkleCmdGetUserContractStateTreeMerkleProof, QSRMerkleCmdGetUserContractTreeMerkleProof, QSRMerkleCmdGetUserTreeMerkleProof,
        },
        cmd_processor::{DPNReadOtherUserLeafMerkleProof, PsyReadCommandProcessorSync, PsyReadCommandProcessorSyncMut},
    },
    ups::{ups_context_input::UserProvingSessionStartContext, ups_standard_cfc_input::UPSCFCStandardStateDeltaInput},
};

pub struct PsyLocalProvingSessionStore<F: RichField, R: PsyReadCommandProcessorSync<F> + Send + Sync> {
    pub cmd_store: PsyCmdStoreWithCache<F, R>,
    pub state_tree_store: KVQSimpleMemoryBackingStore,
    pub active_tx_session_data_store: KVQSimpleMemoryBackingStore,
    pub transaction_records: Vec<PsyLocalTransactionRecord<F>>,

    pub deferred_tx_debt_store: TransactionDebtTreeRef<
        KVQSimpleMemoryBackingStore,
        DPNProvingSessionSimpleMethodCall<F>,
        F,
        DEFERRED_TRANSACTION_TREE_HEIGHT,
        LPS_DEFERRED_TRANSACTION_TREE_ID,
    >,

    //pub delta_merkle_proof_cache: Vec<PsyLocalStateSet<F>>,
    active_transaction_record: PsyLocalTransactionRecord<F>,

    local_state_tracker: PsyLocalStateTracker<F>,

    start_checkpoint: F,
    write_checkpoint: F,
    //current_contract_id: F,
    start_checkpoint_u64: u64,
    write_checkpoint_u64: u64,
    user_id: F,
    user_id_u64: u64,
    nonce: F,
    session_proof_tree_root: QHashOut<F>,

    session_proof_tree_height: usize,
    // replace with the correct implementation of the proof tree
}

// read helpers
#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<F: RichField, R: PsyReadCommandProcessorSync<F> + Send + Sync> PsyLocalProvingSessionStore<F, R> {
    pub fn get_current_contract_id(&self) -> F {
        self.active_transaction_record.call_data.call_data.contract_id
    }
    pub fn get_current_caller_contract_id(&self) -> F {
        self.active_transaction_record.call_data.call_data.caller_contract_id
    }
    pub fn get_start_contract_state_roots(&self) -> Vec<(u64, QHashOut<F>)> {
        let mut mapping = HashMap::<u64, QHashOut<F>>::new();
        for t in self.transaction_records.iter() {
            let c_id = t.user_contract_tree_update_proof.index;
            if !mapping.contains_key(&c_id) {
                mapping.insert(c_id, t.user_contract_tree_update_proof.old_value);
            }
        }

        mapping.into_iter().map(|(k, v)| (k, v)).collect()
    }
    pub fn get_total_slots_modified(&self) -> F {
        F::from_canonical_u32(self.local_state_tracker.total_slots_modified)
    }
    pub fn get_current_method_id(&self) -> F {
        self.active_transaction_record.call_data.call_data.method_id
    }

    pub fn get_current_user_id(&self) -> F {
        self.user_id
    }
    pub fn get_current_user_id_64(&self) -> u64 {
        self.user_id.to_canonical_u64()
    }

    pub fn get_current_start_checkpoint_id(&self) -> F {
        self.start_checkpoint
    }
    pub fn get_current_start_checkpoint_id_u64(&self) -> u64 {
        self.start_checkpoint_u64
    }

    pub fn get_current_write_checkpoint_id(&self) -> F {
        self.write_checkpoint
    }
    pub fn get_current_write_checkpoint_id_u64(&self) -> u64 {
        self.write_checkpoint_u64
    }

    pub fn get_nonce(&self) -> F {
        self.nonce
    }
    pub fn get_nonce_u64(&self) -> u64 {
        self.nonce.to_canonical_u64()
    }
    pub fn get_q_recursion_proof_tree_height(&self) -> usize {
        self.session_proof_tree_height
    }
    pub fn get_q_recursion_proof_tree_root(&self) -> QHashOut<F> {
        self.session_proof_tree_root
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<F: RichField, R: PsyReadCommandProcessorSync<F> + Send + Sync> PsyLocalProvingSessionStore<F, R> {
    pub fn new_at(read_store: R, start_checkpoint: F, user_id: F, nonce: F, q_recursion_tree_height: usize) -> Self {
        let cmd_store = PsyCmdStoreWithCache::new(start_checkpoint.to_canonical_u64(), read_store);

        Self::new_at_with_cmd_store(cmd_store, start_checkpoint, user_id, nonce, q_recursion_tree_height)
    }
    pub fn new_at_with_cmd_store(
        cmd_store: PsyCmdStoreWithCache<F, R>,
        start_checkpoint: F,
        user_id: F,
        nonce: F,
        q_recursion_tree_height: usize,
    ) -> Self {
        Self {
            cmd_store,
            state_tree_store: KVQSimpleMemoryBackingStore::new(),
            active_tx_session_data_store: KVQSimpleMemoryBackingStore::new(),
            local_state_tracker: PsyLocalStateTracker::new(),
            deferred_tx_debt_store: TransactionDebtTreeRef::new(start_checkpoint.to_canonical_u64()),
            transaction_records: Vec::new(),
            //delta_merkle_proof_cache: Vec::new(),
            start_checkpoint: start_checkpoint,
            write_checkpoint: start_checkpoint + F::ONE,
            //current_contract_id: contract_id,
            user_id,
            start_checkpoint_u64: start_checkpoint.to_canonical_u64(),
            write_checkpoint_u64: start_checkpoint.to_canonical_u64() + 1,
            user_id_u64: user_id.to_canonical_u64(),
            //user_id_u32: user_id.to_canonical_u64() as u32,
            nonce,
            active_transaction_record: Default::default(),
            session_proof_tree_height: q_recursion_tree_height,
            session_proof_tree_root: QHashOut::ZERO,
        }
    }
    pub fn into_cmd_store(self) -> PsyCmdStoreWithCache<F, R> {
        self.cmd_store
    }
    pub async fn into_clean_for_user(mut self, user_id: F) -> anyhow::Result<Self> {
        let blk_state = self.cmd_store.resolve_get_latest_block_state_mut().await?;

        let start_checkpoint = F::from_canonical_u64(blk_state.checkpoint_id);
        let nonce = self
            .cmd_store
            .resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData {
                checkpoint_id: blk_state.checkpoint_id,
                user_id: user_id.to_canonical_u64(),
            })
            .await?
            .nonce;

        Ok(self.into_clean_for_user_at_checkpoint(user_id, nonce, start_checkpoint))
    }
    pub fn into_clean_for_user_at_checkpoint(self, user_id: F, nonce: F, start_checkpoint: F) -> Self {
        let q_recursion_tree_height = self.session_proof_tree_height;
        let cmd_store = self.into_cmd_store();

        Self::new_at_with_cmd_store(cmd_store, start_checkpoint, user_id, nonce, q_recursion_tree_height)
    }
    pub fn set_proof_tree_root(&mut self, session_proof_tree_root: QHashOut<F>) {
        self.session_proof_tree_root = session_proof_tree_root;
    }
    pub async fn new_at_head(read_store: R, user_id: F, nonce: F, q_recursion_tree_height: usize) -> anyhow::Result<Self> {
        let start_checkpoint = read_store.resolve_get_latest_block_state().await?;

        Ok(Self::new_at(
            read_store,
            F::from_noncanonical_u64(start_checkpoint.checkpoint_id),
            user_id,
            nonce,
            q_recursion_tree_height,
        ))
    }
    pub fn clear(&mut self) {
        self.cmd_store.clear_cache_mut();
        self.state_tree_store.clear();
        self.write_checkpoint = self.start_checkpoint + F::ONE;
        self.write_checkpoint_u64 = self.start_checkpoint_u64 + 1;
    }
    pub fn get_latest_deferred_tx_item(&self) -> Option<&DPNTransactionDebtItem<DPNProvingSessionSimpleMethodCall<F>, F>> {
        self.deferred_tx_debt_store.get_latest_proof_debt_item()
    }
}

type GF = GoldilocksField;

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<R: PsyReadCommandProcessorSync<GoldilocksField> + Send + Sync> PsyLocalProvingSessionStore<GoldilocksField, R> {
    pub fn get_deferred_tx_debt_latest_index(&self) -> u64 {
        self.deferred_tx_debt_store.get_latest_index()
    }
    pub fn get_deferred_tx_debt_next_index(&self) -> u64 {
        self.deferred_tx_debt_store.get_next_index()
    }
    pub fn get_inline_tx_debt_latest_index(&self) -> u64 {
        0
    }
    pub fn get_inline_tx_debt_next_index(&self) -> u64 {
        0
    }
    pub fn get_deferred_tx_tree_leaf(&self, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        self.deferred_tx_debt_store
            .get_tx_debt_leaf(&self.active_tx_session_data_store, leaf_index)
    }
    pub fn get_inline_tx_tree_leaf(&self, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        Ok(SimpleMerkleTree::<PsyHasher, QHashOut<GF>>::new(INLINE_TRANSACTION_TREE_HEIGHT).get_leaf(leaf_index))
    }
    pub async fn init_transaction(&mut self, call_data: DPNProvingSessionSimpleMethodCall<GF>) -> anyhow::Result<()> {
        let uct_proof = self
            .get_self_user_contract_tree_leaf(call_data.contract_id)
            .await?
            .to_delta_merkle_proof_inplace();
        let start_contract_state_tree_root = if uct_proof.old_value.eq(&QHashOut::ZERO) {
            let state_tree_height = self
                .cmd_store
                .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData {
                    contract_id: call_data.contract_id.to_canonical_u64(),
                })
                .await?
                .state_tree_height
                .to_canonical_u64() as usize;
            PsyHasher::get_zero_hash(state_tree_height)
        } else {
            uct_proof.old_value
        };

        let record = PsyLocalTransactionRecord {
            start_checkpoint: self.start_checkpoint,
            write_checkpoint: self.write_checkpoint,
            call_data: DPNProvingSessionSignableMethodCall {
                checkpoint_id: self.start_checkpoint,
                user_id: self.user_id,
                call_data,
            },
            start_contract_state_tree_root,
            end_contract_state_tree_root: start_contract_state_tree_root,
            contract_state_tree_update_proofs: Vec::new(),
            user_contract_tree_update_proof: uct_proof,
            added_deferred_tx_items: Vec::new(),
        };

        self.active_transaction_record = record;

        Ok(())
    }
    /*
    pub fn get_full_data(&mut self, contract_id: GF, method_id: GF, inputs: &[GF], outputs: &[GF]) -> anyhow::Result<DapenCFCUserTransactionCallStartContext<GF>> {
    }*/

    pub async fn get_call_start_data(
        &mut self,
        contract_id: GF,
        method_id: GF,
        inputs: &[GF],
    ) -> anyhow::Result<DapenCFCUserTransactionCallStartContext<GF>> {
        let contract_state_root_proof = self.get_self_user_contract_tree_leaf(contract_id).await?;

        let start_user_contract_tree_root = contract_state_root_proof.root;
        let start_contract_state_tree_root = if contract_state_root_proof.value.eq(&QHashOut::ZERO) {
            let state_tree_height = self
                .cmd_store
                .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData {
                    contract_id: contract_id.to_canonical_u64(),
                })
                .await?
                .state_tree_height
                .to_canonical_u64() as usize;
            PsyHasher::get_zero_hash(state_tree_height)
        } else {
            contract_state_root_proof.value
        };

        let call_data = DPNProvingSessionCompactMethodCall {
            caller_contract_id: self.active_transaction_record.call_data.call_data.caller_contract_id,
            contract_id,
            method_id,
            inputs_length: GF::from_canonical_u64(inputs.len() as u64),
            inputs_hash: safe_hash_fixed_length::<PsyHasher, GF>(inputs),
        };

        let start_deferred_tx_debt_tree_root = self.get_latest_deferred_tx_leaf()?.root;
        let start_user_balance = GF::ZERO;
        let start_user_event_index = GF::ZERO;

        Ok(DapenCFCUserTransactionCallStartContext {
            start_user_contract_tree_root,
            start_contract_state_tree_root,
            call_data,
            start_deferred_tx_debt_tree_root,
            start_user_balance,
            start_user_event_index,
        })
    }
    pub async fn get_global_state_tree_roots(&mut self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointGlobalStateRoots<GF>> {
        let contract_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetContractTreeRoot(QSRHashCmdGetContractTreeRoot { checkpoint_id }))
            .await?;

        let deposit_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetDepositTreeRoot(QSRHashCmdGetDepositTreeRoot { checkpoint_id }))
            .await?;
        let user_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetUserTreeRoot(QSRHashCmdGetUserTreeRoot { checkpoint_id }))
            .await?;
        let withdrawal_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetWithdrawalTreeRoot(QSRHashCmdGetWithdrawalTreeRoot { checkpoint_id }))
            .await?;
        let user_registration_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetUserRegistrationTreeRoot(QSRHashCmdGetUserRegistrationTreeRoot {
                checkpoint_id,
            }))
            .await?;

        Ok(PsyCheckpointGlobalStateRoots {
            contract_tree_root,
            deposit_tree_root,
            user_tree_root,
            withdrawal_tree_root,
            user_registration_tree_root,
        })
    }
    pub async fn get_fresh_start_ctx_for_user(&mut self, user: GF) -> anyhow::Result<DapenCFCProvingSessionStartContext<GF>> {
        let checkpoint_id = self.start_checkpoint_u64;
        let checkpoint_leaf = self
            .cmd_store
            .resolve_get_checkpoint_leaf_mut(&QSRCmdGetCheckpointLeafData { checkpoint_id })
            .await?;
        let checkpoint_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetCheckpointTreeRoot(QSRHashCmdGetCheckpointTreeRoot { checkpoint_id }))
            .await?;
        let state_roots = self.get_global_state_tree_roots(checkpoint_id).await?;

        let user_leaf = self
            .cmd_store
            .resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData {
                checkpoint_id: self.start_checkpoint_u64 + 1000,
                user_id: user.to_canonical_u64(),
            })
            .await?;

        if user_leaf.last_checkpoint_id.to_canonical_u64() > checkpoint_id {
            anyhow::bail!(
                "the user's checkpoint is ahead of the proving session (user sync'd to {}, proving session on checkpoint {})",
                user_leaf.last_checkpoint_id.to_canonical_u64(),
                checkpoint_id
            );
        }

        let res = DapenCFCProvingSessionStartContext {
            checkpoint_id: GF::from_canonical_u64(checkpoint_id),
            checkpoint_tree_root,
            checkpoint_leaf,
            state_roots,
            start_session_user_leaf: user_leaf,
        };

        tracing::debug!("DapenCFCProvingSessionStartContext: {}", serde_json::to_string_pretty(&res).unwrap());

        Ok(res)
    }
    async fn set_contract_state_slot_inner(
        &mut self,
        contract: GF,
        slot: GF,
        value: QHashOut<GF>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let state_tree_height = self
            .cmd_store
            .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData {
                contract_id: contract.to_canonical_u64(),
            })
            .await?
            .state_tree_height
            .to_canonical_u64() as u8;
        let id = UserContractStateTreeId::<KVQSimpleMemoryBackingStore>::new(self.user_id_u64, contract.to_canonical_u64() as u32, state_tree_height);
        let base_mp = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                    checkpoint_id: self.start_checkpoint_u64,
                    user_id: self.user_id_u64,
                    contract_id: contract.to_canonical_u64() as u32,
                    height: state_tree_height,
                    leaf_id: slot.to_canonical_u64(),
                },
            ))
            .await?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, self.start_checkpoint_u64, &base_mp)?;
        let dmp = id.set_leaf_ucs(&mut self.state_tree_store, self.write_checkpoint_u64, slot.to_canonical_u64(), value)?;
        /*let cache_value = PsyLocalStateSet{
            contract,
            slot,
            contract_state_transition_proof: dmp.clone(),
        };
        self.delta_merkle_proof_cache.push(cache_value);*/

        Ok(dmp)
        // self.state_tree_store.map.insert((self.user_id_u64, contract, slot),
        // value);
    }
    pub async fn set_contract_state_slot(
        &mut self,
        contract: GF,
        slot: GF,
        value: QHashOut<GF>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        tracing::debug!("Proving session - slot: {}, value: {}", slot, value);
        let result = self.set_contract_state_slot_inner(contract, slot, value).await?;
        self.local_state_tracker.notify_update_slot_dmp(contract.to_canonical_u64(), &result);
        Ok(result)
    }
    pub async fn get_contract_state_slot(&mut self, contract: GF, slot: GF) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        let state_tree_height = self
            .cmd_store
            .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData {
                contract_id: contract.to_canonical_u64(),
            })
            .await?
            .state_tree_height
            .to_canonical_u64() as u8;
        let id = UserContractStateTreeId::<KVQSimpleMemoryBackingStore>::new(self.user_id_u64, contract.to_canonical_u64() as u32, state_tree_height);
        let base_mp = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                    checkpoint_id: self.start_checkpoint_u64,
                    user_id: self.user_id_u64,
                    contract_id: contract.to_canonical_u64() as u32,
                    height: state_tree_height,
                    leaf_id: slot.to_canonical_u64(),
                },
            ))
            .await?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, self.start_checkpoint_u64, &base_mp)?;
        id.get_leaf_ucs(&self.state_tree_store, self.write_checkpoint_u64, slot.to_canonical_u64())
        // self.state_tree_store.map.insert((self.user_id_u64, contract, slot),
        // value);
    }
    pub async fn get_self_user_contract_tree_leaf(&mut self, contract_id: GF) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        let old_upper_merkle_proof = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(
                QSRMerkleCmdGetUserContractTreeMerkleProof {
                    checkpoint_id: self.start_checkpoint_u64,
                    user_id: self.user_id_u64,
                    contract_id: contract_id.to_canonical_u64() as u32,
                },
            ))
            .await?;

        UserContractTreeStore::<KVQSimpleMemoryBackingStore>::injest_merkle_proof_sfc(
            &mut self.state_tree_store,
            self.user_id_u64,
            self.start_checkpoint_u64,
            &old_upper_merkle_proof,
        )?;
        UserContractTreeStore::<KVQSimpleMemoryBackingStore>::get_leaf_sfc(
            &self.state_tree_store,
            self.write_checkpoint_u64,
            self.user_id_u64,
            contract_id.to_canonical_u64(),
        )
    }
    async fn set_user_contract_tree_leaf(&mut self, contract_id: GF, leaf: QHashOut<GF>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        //let latest_root = self.get_contract_state_slot(contract_id, GF::ZERO)?.root;

        let old_upper_merkle_proof = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(
                QSRMerkleCmdGetUserContractTreeMerkleProof {
                    checkpoint_id: self.start_checkpoint_u64,
                    user_id: self.user_id_u64,
                    contract_id: contract_id.to_canonical_u64() as u32,
                },
            ))
            .await?;

        UserContractTreeStore::<KVQSimpleMemoryBackingStore>::injest_merkle_proof_sfc(
            &mut self.state_tree_store,
            self.user_id_u64,
            self.start_checkpoint_u64,
            &old_upper_merkle_proof,
        )?;
        UserContractTreeStore::<KVQSimpleMemoryBackingStore>::set_leaf_sfc(
            &mut self.state_tree_store,
            self.write_checkpoint_u64,
            self.user_id_u64,
            contract_id.to_canonical_u64(),
            leaf,
        )
    }

    async fn update_contract_state_root_in_user_contract_tree(&mut self, contract_id: GF) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let latest_root = self.get_contract_state_slot(contract_id, GF::ZERO).await?.root;

        let old_upper_merkle_proof = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(
                QSRMerkleCmdGetUserContractTreeMerkleProof {
                    checkpoint_id: self.start_checkpoint_u64,
                    user_id: self.user_id_u64,
                    contract_id: contract_id.to_canonical_u64() as u32,
                },
            ))
            .await?;

        UserContractTreeStore::<KVQSimpleMemoryBackingStore>::injest_merkle_proof_sfc(
            &mut self.state_tree_store,
            self.user_id_u64,
            self.start_checkpoint_u64,
            &old_upper_merkle_proof,
        )?;
        UserContractTreeStore::<KVQSimpleMemoryBackingStore>::set_leaf_sfc(
            &mut self.state_tree_store,
            self.write_checkpoint_u64,
            self.user_id_u64,
            contract_id.to_canonical_u64(),
            latest_root,
        )
    }
    pub async fn get_external_user_leaf_proof(&mut self, user_id: GF) -> anyhow::Result<DPNReadOtherUserLeafMerkleProof<GF>> {
        let user_tree_proof = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserTreeMerkleProof(QSRMerkleCmdGetUserTreeMerkleProof {
                checkpoint_id: self.start_checkpoint_u64,
                user_id: user_id.to_canonical_u64(),
            }))
            .await?;
        let user_leaf = self
            .cmd_store
            .resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData {
                checkpoint_id: self.start_checkpoint_u64,
                user_id: user_id.to_canonical_u64(),
            })
            .await?;
        Ok(DPNReadOtherUserLeafMerkleProof { user_tree_proof, user_leaf })
    }
    pub fn add_deferred_tx_to_debt(&mut self, tx: DPNProvingSessionSimpleMethodCall<GF>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let insertion_result = self.deferred_tx_debt_store.add_tx_debt(&mut self.active_tx_session_data_store, tx)?;
        let tx_debt_item = self.deferred_tx_debt_store.get_latest_proof_debt_item().unwrap().to_owned();
        self.active_transaction_record.add_deferred_tx_item(tx_debt_item);

        Ok(insertion_result)
    }
    pub fn get_deferred_tx_leaf(&self, leaf_index: GF) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        self.deferred_tx_debt_store
            .get_tx_debt_leaf(&self.active_tx_session_data_store, leaf_index.to_canonical_u64())
    }
    pub fn get_latest_deferred_tx_leaf(&self) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        self.deferred_tx_debt_store.get_latest_tx_debt_leaf(&self.active_tx_session_data_store)
    }
    pub fn repay_deferred_tx_debt(
        &mut self,
        tree_leaf_index: u64,
    ) -> anyhow::Result<(
        DPNTransactionDebtItem<DPNProvingSessionSimpleMethodCall<GF>, GF>,
        DeltaMerkleProofCore<QHashOut<GF>>,
    )> {
        self.deferred_tx_debt_store
            .repay_tx_debt(&mut self.active_tx_session_data_store, tree_leaf_index)
    }

    pub async fn finalize_transaction(&mut self) -> anyhow::Result<()> {
        let contract_id = self.active_transaction_record.call_data.call_data.contract_id;

        let uct_proof = self.update_contract_state_root_in_user_contract_tree(contract_id).await?;
        self.active_transaction_record.set_uct_proof(uct_proof);

        /*
         * [FIXED?] TODO/PERF: avoid clone as we just want to move this to
         * transaction records let record_for_storage =
         * self.active_transaction_record.clone();
         * self.active_transaction_record = PsyLocalTransactionRecord::default();
         * self.transaction_records.push(record_for_storage);
         */

        // Temporarily take ownership of `active_transaction_record`
        let active_record = std::mem::take(&mut self.active_transaction_record);
        // Move the active record into `transaction_records`
        self.transaction_records.push(active_record);
        // Reset `active_transaction_record` to its default state
        self.active_transaction_record = PsyLocalTransactionRecord::default();

        Ok(())
    }
    pub async fn get_ups_start_ctx(&mut self) -> anyhow::Result<UserProvingSessionStartContext<GF>> {
        let checkpoint_id = self.start_checkpoint_u64;
        let checkpoint_leaf = self
            .cmd_store
            .resolve_get_checkpoint_leaf_mut(&QSRCmdGetCheckpointLeafData { checkpoint_id })
            .await?;
        let checkpoint_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetCheckpointTreeRoot(QSRHashCmdGetCheckpointTreeRoot { checkpoint_id }))
            .await?;

        let user_leaf = self
            .cmd_store
            .resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData {
                checkpoint_id: self.start_checkpoint_u64 + 1000,
                user_id: self.user_id.to_canonical_u64(),
            })
            .await?;
        let start_ctx = UserProvingSessionStartContext::<GF> {
            checkpoint_id: self.start_checkpoint,
            checkpoint_tree_root,
            checkpoint_leaf_hash: checkpoint_leaf.qfhash::<PsyHasher>(),
            start_session_user_leaf: user_leaf,
        };
        tracing::debug!("ups_start_ctx: {}", serde_json::to_string_pretty(&start_ctx).unwrap());
        Ok(start_ctx)
    }

    pub async fn get_contract_inclusion_proof(&mut self, contract_id: u32) -> anyhow::Result<PsyContractInclusionProof<GF>> {
        let contract_leaf = self
            .cmd_store
            .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData {
                contract_id: contract_id as u64,
            })
            .await?;
        let contract_tree_merkle_proof = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetContractTreeMerkleProof(QSRMerkleCmdGetContractTreeMerkleProof {
                checkpoint_id: self.start_checkpoint_u64,
                contract_id: contract_id,
            }))
            .await?;

        Ok(PsyContractInclusionProof {
            contract_leaf,
            contract_tree_merkle_proof,
        })
    }

    pub async fn get_contract_function_inclusion_proof(
        &mut self,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<PsyContractFunctionInclusionProof<GF>> {
        let contract_inclusion_proof = self.get_contract_inclusion_proof(contract_id).await?;
        let contract_function_merkle_proof = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetContractFunctionTreeMerkleProof(
                QSRMerkleCmdGetContractFunctionTreeMerkleProof {
                    checkpoint_id: self.start_checkpoint_u64,
                    contract_id,
                    function_id,
                },
            ))
            .await?;

        Ok(PsyContractFunctionInclusionProof {
            contract_inclusion_proof,
            contract_function_merkle_proof,
        })
    }

    pub async fn get_all_state_updates(&mut self) -> anyhow::Result<(Vec<PsyContractStateUpdateHistory<GF>>, u32)> {
        let total_slots_modified = self.local_state_tracker.total_slots_modified;
        let tracker_results = self.local_state_tracker.get_results();

        for r in tracker_results.iter() {
            let c = GF::from_canonical_u64(r.contract_id);
            for slot in r.slots.iter() {
                self.set_contract_state_slot_inner(c, GF::from_canonical_u64(slot.index), slot.start_value)
                    .await?;
            }
            //self.set_user_contract_tree_leaf(c, r.start_state_root)?;
            self.update_contract_state_root_in_user_contract_tree(c).await?;
        }

        let start_state_roots = self.get_start_contract_state_roots();
        //println!("start_state_roots: {}",
        // serde_json::to_string_pretty(&start_state_roots).unwrap());

        for (c, h) in start_state_roots.into_iter() {
            self.set_user_contract_tree_leaf(GF::from_canonical_u64(c), h).await?;
        }

        // let records = self.transaction_records.iter().map(|x|
        // (x.call_data.call_data.contract_id,
        // x.user_contract_tree_update_proof.old_value)).collect::<Vec<_>>();

        //let gt= self.get_self_user_contract_tree_leaf(GF::ZERO)?;
        //println!("gt.root: {:?} ({})",gt.root, serde_json::to_string(&gt).unwrap());

        let mut update_results = Vec::with_capacity(tracker_results.len());

        for r in tracker_results.iter() {
            let mut contract_state_tree_updates: Vec<DeltaMerkleProofCore<QHashOut<GF>>> = Vec::with_capacity(r.slots.len());

            let c = GF::from_canonical_u64(r.contract_id);
            for slot in r.slots.iter() {
                contract_state_tree_updates.push(
                    self.set_contract_state_slot_inner(c, GF::from_canonical_u64(slot.index), slot.end_value)
                        .await?,
                );
            }
            let user_contract_tree_update_proof = self.update_contract_state_root_in_user_contract_tree(c).await?;
            update_results.push(PsyContractStateUpdateHistory {
                user_contract_tree_update_proof,
                contract_state_tree_updates,
            })
        }
        Ok((update_results, total_slots_modified))
    }

    pub fn get_state_delta_input(&mut self) -> anyhow::Result<UPSCFCStandardStateDeltaInput<GF>> {
        todo!()
    }

    pub async fn get_checkpoint_state_roots(&mut self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointGlobalStateRoots<GF>> {
        let user_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetUserTreeRoot(QSRHashCmdGetUserTreeRoot { checkpoint_id }))
            .await?;

        let contract_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetContractTreeRoot(QSRHashCmdGetContractTreeRoot { checkpoint_id }))
            .await?;

        let deposit_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetDepositTreeRoot(QSRHashCmdGetDepositTreeRoot { checkpoint_id }))
            .await?;

        let withdrawal_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetWithdrawalTreeRoot(QSRHashCmdGetWithdrawalTreeRoot { checkpoint_id }))
            .await?;

        let user_registration_tree_root = self
            .cmd_store
            .resolve_get_hash_mut(&QSRHashCmd::GetUserRegistrationTreeRoot(QSRHashCmdGetUserRegistrationTreeRoot {
                checkpoint_id,
            }))
            .await?;

        Ok(PsyCheckpointGlobalStateRoots {
            contract_tree_root,
            deposit_tree_root,
            user_tree_root,
            withdrawal_tree_root,
            user_registration_tree_root,
        })
    }

    pub async fn notify_clear_entire_tree(&mut self, contract_id: u64) -> anyhow::Result<()> {
        if let Some(contract_result) = self.local_state_tracker.get_contract_result(contract_id) {
            for slot in contract_result.slots.iter() {
                self.set_contract_state_slot(GF::from_canonical_u64(contract_id), GF::from_canonical_u64(slot.index), QHashOut::ZERO)
                    .await?;
            }
        }
        Ok(())
    }
}
