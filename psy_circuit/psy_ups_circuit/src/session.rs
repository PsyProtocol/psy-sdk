use plonky2::{
    field::{
        extension::Extendable,
        goldilocks_field::GoldilocksField,
        packed::PackedField,
        types::{Field, PrimeField64},
    },
    hash::hash_types::{HashOut, RichField},
    plonk::{
        circuit_data::VerifierOnlyCircuitData,
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common_circuit::{
    circuits::traits::qstandard::QStandardCircuit, treeprover::qrecursion::standard::manager::portable::core::PortableQTreeRecursionManager,
};
use psy_core::{
    config::network_constants::{
        DEFAULT_CALLER_CONTRACT_ID_U64, DEFERRED_TRANSACTION_TREE_HEIGHT, GUTA_FEE, INLINE_TRANSACTION_TREE_HEIGHT, TOKEN_CONTRACT_ID,
        TOKEN_SIMPLE_BURN_METHOD_ID, UPS_SESSION_PROOF_TREE_HEIGHT,
    },
    data::qhashout::QHashOut,
    ups::circuits::LocalCircuitType,
    utils::debug_timer::DebugTimer,
};
use psy_crypto::{
    common::witnesses::qrecursion::{
        header::{AttestProofInTreeInput, AttestTreeAwareProofInTreeInput},
        proof_data::{InputLeafProof, TreeAwareTreeProofRecord},
    },
    hash::traits::{
        hasher::{FieldQHasher, MerkleZeroHasher},
        qhashable::QFieldHashable,
    },
};
use psy_data::{
    config::store_config::PsyHasher,
    dpn::proving_session::{
        DPNProvingSessionCompactMethodCall, DPNProvingSessionSimpleMethodCall, DPNTransactionDebtItem, PsyLocalTransactionRecord,
    },
    guta::{api::SubmitUserEndCapNonProofCoreInput, end_cap_input::SubmitUserEndCapNonProofInput, stats::GUTAStats},
    qdata::{
        checkpoint::{PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf, PsyCheckpointLeafCompact, PsyCheckpointLeafCompactWithStateRoots},
        ups_end_cap_result::UPSEndCapResultCompact,
        ups_signature::PsyUserProvingSessionSignatureDataCompact,
        user::PsyUserLeaf,
        user_contract_state::{SignContext, UserContractState},
    },
    qstore::{
        controllers::{
            proving_session::PsyLocalProvingSessionStore, session_info::SessionCircuitInfoStore, state_tracker::PsyUserSessionUpdateHistory,
        },
        imm::{
            cache::PsyCmdStoreWithCache,
            cmd::{
                QSRCmdGetCheckpointLeafData, QSRCmdGetContractCodeDefinition, QSRMerkleCmd, QSRMerkleCmdGetCheckpointTreeMerkleProof,
                QSRMerkleCmdGetUserTreeMerkleProof,
            },
            cmd_processor::{PsyReadCommandProcessorSync, PsyReadCommandProcessorSyncMut},
        },
    },
    ups::{
        start_step::UPSStartStepInput,
        ups_cfc_standard_step::{UPSCFCDeferredTransactionCircuitInput, UPSCFCStandardTransactionCircuitInput},
        ups_context_input::{UserProvingSessionCurrentState, UserProvingSessionHeader},
        ups_end_cap::UPSEndCapFromProofTreeGadgetInput,
        ups_standard_cfc_input::{UPSCFCStandardStateDeltaInput, UPSVerifyCFCStandardStepInput, UPSVerifyPopDeferredTxStepInput},
        verify_previous_ups_step::VerifyPreviousUPSStepProofInProofTreeInput,
    },
};
use psy_dpn_circuit::circuits::cfc::DapenContractFunctionCircuit;
use psy_rust_sdk::provider::UPSCircuitManagerTrait;
use psy_vm::{
    dpn::{contract::cfc_code_definition_to_dapen_fc, vm::def::DPNFunctionCircuitDefinition},
    vm::{cfc_input::DapenContractFunctionCircuitInput, exec::PsyEvalSessionResult},
};
use serde::Serialize;

use super::circuit_manager::core::PsyUPSStepCircuitManager;
use crate::circuit_manager::core::QCircuitManager;

const UPS_STEP_LEAF_TYPE: u64 = 1;
const CFC_LEAF_TYPE: u64 = 2;
const ZK_SIG_LEAF_TYPE: u64 = 3;

pub struct UserProvingSessionManager<
    F: RichField + Extendable<D>,
    H: MerkleZeroHasher<QHashOut<F>> + MerkleZeroHasher<HashOut<F>> + AlgebraicHasher<F>,
    R: PsyReadCommandProcessorSync<F> + Send + Sync,
    C: GenericConfig<D, F = F, Hasher = H>,
    const D: usize,
> {
    pub lps: PsyLocalProvingSessionStore<F, R>,
    circuit_info: SessionCircuitInfoStore<F>,
    pub proof_tree_state: PortableQTreeRecursionManager<C, D>,
    pub current_ups_header: UserProvingSessionHeader<F>,
    pub previous_ups_header: UserProvingSessionHeader<F>,
    current_checkpoint_leaf: PsyCheckpointLeaf<F>,
    current_global_state_roots: PsyCheckpointGlobalStateRoots<F>,
    last_ups_step_proof_info: TreeAwareTreeProofRecord<F>,

    tx_log: Vec<DPNProvingSessionSimpleMethodCall<F>>,
}

type F = GoldilocksField;
const D: usize = 2;

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<
        H: MerkleZeroHasher<QHashOut<F>> + MerkleZeroHasher<HashOut<F>> + AlgebraicHasher<F> + FieldQHasher<F>,
        R: PsyReadCommandProcessorSync<F> + Send + Sync,
        C: GenericConfig<D, F = F, Hasher = H> + Serialize,
    > UserProvingSessionManager<F, H, R, C, D>
{
    pub fn into_cmd_store(self) -> PsyCmdStoreWithCache<F, R> {
        self.lps.into_cmd_store()
    }

    pub async fn into_clean_for_user(self, user_id: F) -> anyhow::Result<Self> {
        let ups_step_circuit_whitelist_root = self.current_ups_header.ups_step_circuit_whitelist_root;
        let circuit_info = self.circuit_info;
        let lps = self.lps.into_clean_for_user(user_id).await?;

        Self::new(lps, circuit_info, ups_step_circuit_whitelist_root).await
    }

    pub fn get_checkpoint_state(&self) -> PsyCheckpointLeafCompactWithStateRoots<F> {
        PsyCheckpointLeafCompactWithStateRoots {
            checkpoint_leaf: PsyCheckpointLeafCompact {
                global_chain_root: self.current_checkpoint_leaf.global_chain_root,
                stats_hash: self.current_checkpoint_leaf.stats.qfhash::<H>(),
            },
            global_state_roots: self.current_global_state_roots,
        }
    }

    pub async fn new(
        mut lps: PsyLocalProvingSessionStore<F, R>,
        circuit_info: SessionCircuitInfoStore<F>,
        ups_step_circuit_whitelist_root: QHashOut<F>,
    ) -> anyhow::Result<Self> {
        let proof_tree_state = PortableQTreeRecursionManager::<C, D>::new(UPS_SESSION_PROOF_TREE_HEIGHT as usize).await;
        let session_start_context = lps.get_ups_start_ctx().await?;

        let mut new_user = session_start_context.start_session_user_leaf.clone();

        let latest_checkpoint_id_u64 = lps.get_current_start_checkpoint_id_u64();
        let latest_checkpoint_id_f = lps.get_current_start_checkpoint_id();

        if latest_checkpoint_id_u64 != 0 && latest_checkpoint_id_u64 <= new_user.last_checkpoint_id.to_canonical_u64() {
            anyhow::bail!(
                "Invalid checkpoint: new checkpoint {} must be > last user_leaf checkpoint {}",
                latest_checkpoint_id_u64,
                new_user.last_checkpoint_id.to_canonical_u64()
            );
        }

        new_user.last_checkpoint_id = latest_checkpoint_id_f;
        tracing::debug!("ups_start_checkpoint_id: {}", latest_checkpoint_id_u64);

        let current_checkpoint_leaf = lps
            .cmd_store
            .resolve_get_checkpoint_leaf_mut(&QSRCmdGetCheckpointLeafData {
                checkpoint_id: latest_checkpoint_id_u64,
            })
            .await?;

        let current_global_state_roots = lps.get_global_state_tree_roots(latest_checkpoint_id_u64).await?;

        tracing::debug!(
            "ups_start_global_state_roots: {}",
            serde_json::to_string_pretty(&current_global_state_roots).unwrap()
        );

        let current_state = UserProvingSessionCurrentState {
            user_leaf: new_user,
            deferred_tx_debt_tree_root: H::get_zero_hash(DEFERRED_TRANSACTION_TREE_HEIGHT as usize),
            inline_tx_debt_tree_root: H::get_zero_hash(INLINE_TRANSACTION_TREE_HEIGHT as usize),
            tx_hash_stack: QHashOut::ZERO,
            tx_count: F::ZERO,
        };

        let current_ups_header = UserProvingSessionHeader {
            ups_step_circuit_whitelist_root,
            session_start_context,
            current_state,
        };

        Ok(Self {
            lps,
            proof_tree_state,
            current_ups_header: current_ups_header.clone(),
            previous_ups_header: current_ups_header,
            current_checkpoint_leaf,
            current_global_state_roots,
            last_ups_step_proof_info: TreeAwareTreeProofRecord::default(),
            circuit_info,
            tx_log: vec![],
        })
    }

    pub async fn new_dummy(lps: PsyLocalProvingSessionStore<F, R>, circuit_info: SessionCircuitInfoStore<F>) -> anyhow::Result<Self> {
        let proof_tree_state = PortableQTreeRecursionManager::<C, D>::new(UPS_SESSION_PROOF_TREE_HEIGHT as usize).await;

        Ok(Self {
            lps,
            proof_tree_state,
            current_ups_header: UserProvingSessionHeader::default(),
            previous_ups_header: UserProvingSessionHeader::default(),
            current_checkpoint_leaf: PsyCheckpointLeaf::default(),
            current_global_state_roots: PsyCheckpointGlobalStateRoots::default(),
            last_ups_step_proof_info: TreeAwareTreeProofRecord::default(),
            circuit_info,
            tx_log: vec![],
        })
    }

    pub async fn get_ups_start_witness(&mut self) -> anyhow::Result<UPSStartStepInput<F>> {
        let start_checkpoint_id = self.lps.get_current_start_checkpoint_id_u64();
        tracing::info!(
            "resolve checkpoint tree proof at checkpoint {}, leaf checkpoint {}",
            self.lps.get_current_write_checkpoint_id_u64(),
            start_checkpoint_id
        );
        let checkpoint_tree_proof = self
            .lps
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetCheckpointTreeMerkleProof(QSRMerkleCmdGetCheckpointTreeMerkleProof {
                checkpoint_id: start_checkpoint_id,
                leaf_checkpoint_id: start_checkpoint_id,
            }))
            .await?;

        tracing::info!(
            "resolve user tree proof at checkpoint {}, start checkpoint {}, user {}",
            self.lps.get_current_write_checkpoint_id_u64(),
            start_checkpoint_id,
            self.lps.get_current_user_id_64(),
        );
        let user_tree_proof = self
            .lps
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserTreeMerkleProof(QSRMerkleCmdGetUserTreeMerkleProof {
                checkpoint_id: start_checkpoint_id,
                user_id: self.lps.get_current_user_id_64(),
            }))
            .await?;

        let input = UPSStartStepInput {
            ups_header: self.current_ups_header.clone(),
            checkpoint_leaf: self.current_checkpoint_leaf.clone(),
            state_roots: self.current_global_state_roots.clone(),
            checkpoint_tree_proof,
            user_tree_proof,
        };
        Ok(input)
    }

    pub fn append_to_tx_log(&mut self, item: DPNProvingSessionSimpleMethodCall<F>) -> QHashOut<F> {
        let prev_hash_tip = self.current_ups_header.current_state.tx_hash_stack;
        let new_hash_tip = H::q_two_to_one(prev_hash_tip, item.qfhash::<H>());
        self.current_ups_header.current_state.tx_hash_stack = new_hash_tip;
        self.current_ups_header.current_state.tx_count += F::ONE;
        self.tx_log.push(item);
        new_hash_tip
    }

    pub async fn prove_ups_start<CM: UPSCircuitManagerTrait<C, D> + ?Sized>(&mut self, circuit_mgr: &CM) -> anyhow::Result<()> {
        let mut timer = DebugTimer::new("prove_ups_start");
        timer.lap("start");
        tracing::info!("get_ups_start_witness");
        let input = self.get_ups_start_witness().await?;

        timer.lap("gen_witness");
        if !input.checkpoint_tree_proof.verify::<PsyHasher>() {
            tracing::error!(
                "input.checkpoint_tree_proof {}",
                serde_json::to_string_pretty(&input.checkpoint_tree_proof)?
            );
            anyhow::bail!("invalid checkpoint tree proof");
        }

        if !input.user_tree_proof.verify::<PsyHasher>() {
            tracing::error!("input.user_tree_proof {}", serde_json::to_string_pretty(&input.user_tree_proof)?);
            anyhow::bail!("invalid user tree proof");
        }

        if input.ups_header.session_start_context.start_session_user_leaf.qfhash::<PsyHasher>() != input.user_tree_proof.value {
            tracing::error!(
                "input.ups_header.session_start_context.start_session_user_leaf.qfhash::<PsyHasher>()!= input.user_tree_proof.value\n{:?}!= {:?}",
                input
                    .ups_header
                    .session_start_context
                    .start_session_user_leaf
                    .qfhash::<PsyHasher>()
                    .to_string(),
                input.user_tree_proof.value.to_string()
            );
            anyhow::bail!("value doesn't match user leaf");
        }

        tracing::info!("circuit_mgr.ups_start.prove_base start");
        let proof = circuit_mgr.prove_ups_start(&input).await?;
        timer.lap("circuit_mgr.ups_start.prove_base");

        timer.lap("prove_ups_start");
        let known_proof_tree_root = self.proof_tree_state.get_proof_tree_root().await;
        let inner_public_inputs_hash = input.ups_header.qfhash::<H>();

        let last_ups_step_proof_index = self
            .proof_tree_state
            .injest_single_leaf_proof(InputLeafProof {
                leaf_circuit_type: UPS_STEP_LEAF_TYPE,
                fingerprint: circuit_mgr.ups_start_circuit_fingerprint().await?,
                verifier_data: circuit_mgr.ups_start_circuit_verifier_config().await?,
                proof,
            })
            .await;
        self.last_ups_step_proof_info = TreeAwareTreeProofRecord {
            circuit_id: LocalCircuitType::UPSStart.into(),
            inner_public_inputs_hash,
            known_proof_tree_root,
            proof_tree_index: last_ups_step_proof_index,
        };
        self.previous_ups_header = self.current_ups_header.clone();
        self.current_ups_header = input.ups_header;
        timer.lap("injest_single_leaf_proof");

        Ok(())
    }

    pub async fn get_verify_previous_ups_step_proof(&mut self) -> anyhow::Result<VerifyPreviousUPSStepProofInProofTreeInput<F>> {
        let previous_step_header = self.current_ups_header.clone();
        let ups_circuit_whitelist_merkle_proof = self
            .circuit_info
            .get_whitelist_merkle_proof(self.last_ups_step_proof_info.circuit_id)?
            .to_owned();
        let historical_root_proof = match self
            .proof_tree_state
            .find_zero_hash_proof_for_historical_root(self.last_ups_step_proof_info.known_proof_tree_root)
            .await
        {
            Some(p) => p,
            None => anyhow::bail!(
                "could not find historical root proof for root {:?}",
                self.last_ups_step_proof_info.known_proof_tree_root
            ),
        };
        let inclusion_proof = self
            .proof_tree_state
            .get_leaf_merkle_proof(self.last_ups_step_proof_info.proof_tree_index)
            .await;

        let proof_attestation_witness = AttestTreeAwareProofInTreeInput {
            fingerprint: ups_circuit_whitelist_merkle_proof.value,
            inner_public_inputs_hash: self.last_ups_step_proof_info.inner_public_inputs_hash,
            historical_root_proof,
            inclusion_proof,
        };

        // if !proof_attestation_witness.verify::<H>() {
        //     anyhow::bail!("AttestTreeAwareProofInTreeInput verification failed for
        // previous UPS step"); }

        Ok(VerifyPreviousUPSStepProofInProofTreeInput {
            proof_attestation_witness,
            previous_step_header,
            ups_circuit_whitelist_merkle_proof,
        })
    }

    async fn resolve_contract_function(&mut self, contract_id: u64, method_id: u32) -> anyhow::Result<(usize, DPNFunctionCircuitDefinition)> {
        let contract_def = self
            .lps
            .cmd_store
            .resolve_get_contract_code_mut(&QSRCmdGetContractCodeDefinition { contract_id })
            .await?;

        let (fn_id, fn_code_def) = contract_def
            .functions
            .iter()
            .enumerate()
            .find_map(|(fn_id, f)| if f.method_id == method_id { Some((fn_id, f)) } else { None })
            .ok_or_else(|| anyhow::anyhow!("method ({}) not found in contract {}", method_id, contract_id))?;

        let fn_circuit_def = cfc_code_definition_to_dapen_fc(fn_code_def)?;
        Ok((fn_id, fn_circuit_def))
    }

    pub async fn prove_contract_call<CM: UPSCircuitManagerTrait<C, D> + ?Sized>(
        &mut self,
        circuit_mgr: &CM,
        contract_id: F,
        fn_id: u32,
        fn_circuit_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<()> {
        self.prove_standard_call(circuit_mgr, contract_id, fn_id, fn_circuit_def, inputs).await?;
        self.prove_burn_fee(circuit_mgr).await?;
        Ok(())
    }

    pub async fn prove_standard_call<CM: UPSCircuitManagerTrait<C, D> + ?Sized>(
        &mut self,
        circuit_mgr: &CM,
        contract_id: F,
        fn_id: u32,
        fn_circuit_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<()> {
        let deferred_tx_pivot_index = self.lps.get_deferred_tx_debt_latest_index();
        let inline_tx_pivot_index = self.lps.get_inline_tx_debt_latest_index();
        let tx_log_item = DPNProvingSessionSimpleMethodCall {
            caller_contract_id: F::from_canonical_u64(DEFAULT_CALLER_CONTRACT_ID_U64),
            contract_id,
            method_id: F::from_canonical_u32(fn_circuit_def.method_id),
            inputs: inputs.clone(),
        };
        let (fn_circuit_fingerprint, fn_circuit_verifier_data) =
            circuit_mgr.get_contract_method_common_data(contract_id.to_canonical_u64(), fn_id).await?;
        let cfc_proof_input = self.exec_contract_call(contract_id, fn_circuit_def, inputs).await?;
        let cfc_proof = circuit_mgr
            .prove_contract_call(contract_id.to_canonical_u64(), fn_id, &cfc_proof_input)
            .await?;
        let cfc_proof_index = self
            .proof_tree_state
            .injest_single_leaf_proof(InputLeafProof {
                leaf_circuit_type: CFC_LEAF_TYPE,
                fingerprint: fn_circuit_fingerprint,
                verifier_data: fn_circuit_verifier_data,
                proof: cfc_proof,
            })
            .await;
        let cfc_inclusion_proof = self
            .lps
            .get_contract_function_inclusion_proof(contract_id.to_canonical_u64() as u32, fn_id)
            .await?;
        let historical_root_proof = match self
            .proof_tree_state
            .find_zero_hash_proof_for_historical_root(cfc_proof_input.session_proof_tree_root)
            .await
        {
            Some(mp) => mp,
            None => anyhow::bail!("error finding historical root proof in proof_tree_state"),
        };
        let checkpoint_state = self.get_checkpoint_state();
        let last_tx_rec: &PsyLocalTransactionRecord<GoldilocksField> = self.lps.transaction_records.last().unwrap();
        let user_contract_tree_update_proof = last_tx_rec.user_contract_tree_update_proof.clone();
        let deferred_tx_debt_pivot_proof = self.lps.get_deferred_tx_tree_leaf(deferred_tx_pivot_index)?;
        let inline_tx_debt_pivot_proof = self.lps.get_inline_tx_tree_leaf(inline_tx_pivot_index)?;
        let new_step_deferred_tx_debt_tree_root = deferred_tx_debt_pivot_proof.root;
        let new_step_inline_tx_debt_tree_root = inline_tx_debt_pivot_proof.root;
        let proof_tree_inclusion_proof = self.proof_tree_state.get_leaf_merkle_proof(cfc_proof_index).await;
        let new_step_known_proof_tree_root = proof_tree_inclusion_proof.root;
        let verify_cfc_proof_input = AttestTreeAwareProofInTreeInput {
            fingerprint: fn_circuit_fingerprint,
            inner_public_inputs_hash: cfc_proof_input.tx_input_ctx.qfhash::<H>(),
            historical_root_proof,
            inclusion_proof: proof_tree_inclusion_proof,
        };

        // if !verify_cfc_proof_input.verify::<H>() {
        //     anyhow::bail!("AttestTreeAwareProofInTreeInput verification failed for
        // CFC standard step"); }

        let process_cfc_state_delta_input = UPSCFCStandardStateDeltaInput {
            cfc_transaction_input_context: cfc_proof_input.tx_input_ctx,
            user_contract_tree_update_proof,
            deferred_tx_debt_pivot_proof,
            inline_tx_debt_pivot_proof,
        };
        let new_step_user_leaf = PsyUserLeaf {
            public_key: self.current_ups_header.current_state.user_leaf.public_key,
            user_state_tree_root: process_cfc_state_delta_input.user_contract_tree_update_proof.new_root,
            balance: process_cfc_state_delta_input
                .cfc_transaction_input_context
                .transaction_call_start_ctx
                .start_user_balance
                + process_cfc_state_delta_input
                    .cfc_transaction_input_context
                    .transaction_end_ctx
                    .total_balance_spent,
            event_index: process_cfc_state_delta_input
                .cfc_transaction_input_context
                .transaction_call_start_ctx
                .start_user_event_index
                + process_cfc_state_delta_input
                    .cfc_transaction_input_context
                    .transaction_end_ctx
                    .total_events_emitted,
            nonce: self.current_ups_header.current_state.user_leaf.nonce,
            last_checkpoint_id: self.current_ups_header.current_state.user_leaf.last_checkpoint_id,
            user_id: self.current_ups_header.current_state.user_leaf.user_id,
        };
        let tx_log_item_hash = tx_log_item.qfhash::<H>();
        let new_step_tx_hash_stack = H::q_two_to_one(self.current_ups_header.current_state.tx_hash_stack, tx_log_item_hash);
        let new_step_tx_count = self.current_ups_header.current_state.tx_count + F::ONE;
        let new_step_current_state = UserProvingSessionCurrentState {
            user_leaf: new_step_user_leaf,
            deferred_tx_debt_tree_root: new_step_deferred_tx_debt_tree_root,
            inline_tx_debt_tree_root: new_step_inline_tx_debt_tree_root,
            tx_hash_stack: new_step_tx_hash_stack,
            tx_count: new_step_tx_count,
        };
        let ups_cfc_standard_input = UPSVerifyCFCStandardStepInput {
            checkpoint_state,
            verify_cfc_proof_input,
            cfc_inclusion_proof,
            process_cfc_state_delta_input,
        };
        let verify_previous_ups_step = self.get_verify_previous_ups_step_proof().await?;
        let circuit_input = UPSCFCStandardTransactionCircuitInput {
            verify_previous_ups_step,
            standard_cfc_step: ups_cfc_standard_input,
        };
        let ups_proof = circuit_mgr.prove_ups_cfc_standard_tx(&circuit_input).await?;

        self.last_ups_step_proof_info.circuit_id = LocalCircuitType::UPSCFCStandard.into();
        let new_ups_header = UserProvingSessionHeader {
            ups_step_circuit_whitelist_root: self.current_ups_header.ups_step_circuit_whitelist_root,
            session_start_context: self.current_ups_header.session_start_context.clone(),
            current_state: new_step_current_state,
        };
        let ups_step_proof_tree_index = self
            .proof_tree_state
            .injest_single_leaf_proof(InputLeafProof {
                leaf_circuit_type: UPS_STEP_LEAF_TYPE,
                fingerprint: circuit_mgr.ups_cfc_standard_tx_circuit_fingerprint().await?,
                verifier_data: circuit_mgr.ups_cfc_standard_tx_circuit_verifier_config().await?,
                proof: ups_proof,
            })
            .await;
        self.last_ups_step_proof_info = TreeAwareTreeProofRecord {
            inner_public_inputs_hash: new_ups_header.qfhash::<H>(),
            circuit_id: LocalCircuitType::UPSCFCStandard.into(),
            known_proof_tree_root: new_step_known_proof_tree_root,
            proof_tree_index: ups_step_proof_tree_index,
        };
        self.previous_ups_header = self.current_ups_header.clone();
        self.current_ups_header = new_ups_header;
        self.tx_log.push(tx_log_item);

        let deferred_debt_items = self.lps.transaction_records.last().unwrap().added_deferred_tx_items.clone();

        for debt_item in &deferred_debt_items {
            self.repay_deferred_debt(circuit_mgr, debt_item).await?;
        }

        Ok(())
    }

    pub fn get_sighash(&self, network_magic: u64, nonce: F) -> QHashOut<F> {
        let mut end_user_leaf = self.current_ups_header.current_state.user_leaf.clone();
        end_user_leaf.nonce = nonce;

        let sig_data = PsyUserProvingSessionSignatureDataCompact {
            start_user_leaf_hash: self.current_ups_header.session_start_context.start_session_user_leaf.qfhash::<H>(),
            end_user_leaf_hash: end_user_leaf.qfhash::<H>(),
            checkpoint_leaf_hash: self.current_ups_header.session_start_context.checkpoint_leaf_hash,
            tx_stack_hash: self.current_ups_header.current_state.tx_hash_stack,
            tx_count: self.current_ups_header.current_state.tx_count,
        };

        // get checkpoint tree proof
        let sign_context = SignContext {
            checkpoint_tree_root: self.current_ups_header.session_start_context.checkpoint_tree_root,
            user_leaf: self.current_ups_header.current_state.user_leaf,
        };

        let sig_action = sig_data.get_sig_action_for_user::<H>(network_magic, self.lps.get_current_user_id(), nonce, sign_context);

        let sighash = sig_action.get_qhash::<H>();

        sighash
    }

    pub async fn prove_burn_fee<CM: UPSCircuitManagerTrait<C, D> + ?Sized>(&mut self, circuit_mgr: &CM) -> anyhow::Result<()> {
        tracing::info!("Adding burn transaction for GUTA fee: {}", GUTA_FEE);

        let (burn_fn_id, burn_fn_circuit_def) = self
            .resolve_contract_function(TOKEN_CONTRACT_ID as u64, TOKEN_SIMPLE_BURN_METHOD_ID)
            .await?;

        let burn_contract_id = F::from_canonical_u64(TOKEN_CONTRACT_ID as u64);
        let burn_amount = F::from_canonical_u64(GUTA_FEE);
        let burn_inputs = vec![burn_amount];

        tracing::info!("Executing burn transaction: contract_id={}, amount={}", burn_contract_id, burn_amount);

        self.prove_standard_call(circuit_mgr, burn_contract_id, burn_fn_id as u32, &burn_fn_circuit_def, burn_inputs)
            .await?;

        Ok(())
    }

    pub async fn prove_end_cap<CM: UPSCircuitManagerTrait<C, D> + ?Sized>(
        &mut self,
        circuit_mgr: &CM,
        network_magic: u64,
        nonce: F,
        zk_sig_fingerprint: QHashOut<F>,
        public_key_param: QHashOut<F>,
        signature_proof: ProofWithPublicInputs<F, C, D>,
        verifier_data: VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        if signature_proof.public_inputs.len() != 4 {
            anyhow::bail!("signature proof must have 4 public inputs");
        }

        // ensure the signature is correct
        let expected_sighash = self.get_sighash(network_magic, nonce);
        let expected_public_inputs_hash = H::q_two_to_one(expected_sighash, public_key_param);
        tracing::info!(
            "Expected public inputs hash: H::q_two_to_one({}, {}) = {}",
            expected_sighash,
            public_key_param,
            expected_public_inputs_hash
        );

        let proof_public_inputs_hash = QHashOut(HashOut {
            elements: [
                signature_proof.public_inputs[0],
                signature_proof.public_inputs[1],
                signature_proof.public_inputs[2],
                signature_proof.public_inputs[3],
            ],
        });
        tracing::info!(
            "Actual proof public inputs: [{}, {}, {}, {}] = {}",
            signature_proof.public_inputs[0],
            signature_proof.public_inputs[1],
            signature_proof.public_inputs[2],
            signature_proof.public_inputs[3],
            proof_public_inputs_hash
        );
        if !proof_public_inputs_hash.eq(&expected_public_inputs_hash) {
            anyhow::bail!(
                "invalid signature for ups session, likely incorrect sighash\n{:?}!= {:?}",
                proof_public_inputs_hash.to_string(),
                expected_public_inputs_hash.to_string()
            );
        }

        // injest signature into the proof tree
        tracing::info!(
            "injesting zk signature proof into proof tree, fingerprint: {:?}",
            zk_sig_fingerprint.to_string()
        );
        let zk_sig_proof_index = self
            .proof_tree_state
            .injest_single_leaf_proof(InputLeafProof {
                leaf_circuit_type: ZK_SIG_LEAF_TYPE,
                fingerprint: zk_sig_fingerprint,
                proof: signature_proof,
                verifier_data,
            })
            .await;

        // compress all proofs into a sign tree proof
        tracing::info!("compress all proofs into a sign tree proof");
        self.proof_tree_state.finalize_tree(circuit_mgr).await?;

        let zk_sig_leaf_proof = self.proof_tree_state.get_leaf_merkle_proof(zk_sig_proof_index).await;
        let end_cap_from_proof_tree_input = UPSEndCapFromProofTreeGadgetInput {
            verify_previous_ups_step_input: self.get_verify_previous_ups_step_proof().await?,
            verify_zk_signature_proof_input: AttestProofInTreeInput {
                fingerprint: zk_sig_fingerprint,
                public_inputs_hash: proof_public_inputs_hash,
                inclusion_proof: zk_sig_leaf_proof,
            },
            user_public_key_param: public_key_param,
            nonce,
            slots_modified: self.lps.get_total_slots_modified(),
            second_to_last_tx_hash_stack: self.previous_ups_header.current_state.tx_hash_stack,
        };

        let finalized_proof_tree_record = self.proof_tree_state.get_finalized_proot_tree_record().await?;

        let proof = circuit_mgr
            .prove_ups_end_cap(&self.circuit_info, &end_cap_from_proof_tree_input, &finalized_proof_tree_record)
            .await?;

        // update the user's nonce
        self.current_ups_header.current_state.user_leaf.nonce = nonce;

        Ok(proof)
    }

    pub async fn exec_deferred_contract_call(
        &mut self,
        contract_id: F,
        caller_contract_id: F,
        fn_circuit_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<F>> {
        self.lps.set_proof_tree_root(self.proof_tree_state.get_proof_tree_root().await);
        PsyEvalSessionResult::new()
            .exec_deferred_contract_call(&mut self.lps, contract_id, caller_contract_id, fn_circuit_def, inputs)
            .await
    }

    pub async fn exec_contract_call(
        &mut self,
        contract_id: F,
        fn_circuit_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<F>> {
        self.exec_deferred_contract_call(contract_id, F::from_canonical_u64(DEFAULT_CALLER_CONTRACT_ID_U64), fn_circuit_def, inputs)
            .await
    }

    async fn repay_deferred_debt<CM: UPSCircuitManagerTrait<C, D> + ?Sized>(
        &mut self,
        circuit_mgr: &CM,
        initial_debt_item: &psy_data::dpn::proving_session::DPNTransactionDebtItem<DPNProvingSessionSimpleMethodCall<F>, F>,
    ) -> anyhow::Result<()> {
        let mut debt_queue = vec![initial_debt_item.clone()];

        while let Some(debt_item) = debt_queue.pop() {
            self.prove_deferred_call(circuit_mgr, &debt_item).await?;

            let new_debt_items = self.lps.transaction_records.last().unwrap().added_deferred_tx_items.clone();

            debt_queue.extend(new_debt_items);
        }

        Ok(())
    }

    async fn prove_deferred_call<CM: UPSCircuitManagerTrait<C, D> + ?Sized>(
        &mut self,
        circuit_mgr: &CM,
        debt_item: &DPNTransactionDebtItem<DPNProvingSessionSimpleMethodCall<F>, F>,
    ) -> anyhow::Result<()> {
        let (_, debt_removal_proof) = self.lps.repay_deferred_tx_debt(debt_item.tree_index)?;
        let deferred_tx = &debt_item.call_data;
        let method_id = deferred_tx.method_id.to_canonical_u64() as u32;
        let contract_id = deferred_tx.contract_id.to_canonical_u64();
        let (fn_id, fn_circuit_def) = self.resolve_contract_function(contract_id, method_id).await?;
        let cfc_proof_input = self
            .exec_deferred_contract_call(
                deferred_tx.contract_id,
                deferred_tx.caller_contract_id,
                &fn_circuit_def,
                deferred_tx.inputs.clone(),
            )
            .await?;

        let (fn_circuit_fingerprint, fn_circuit_verifier_data) = circuit_mgr
            .get_contract_method_common_data(deferred_tx.contract_id.to_canonical_u64(), fn_id as u32)
            .await?;
        let cfc_proof = circuit_mgr
            .prove_contract_call(deferred_tx.contract_id.to_canonical_u64(), fn_id as u32, &cfc_proof_input)
            .await?;
        let cfc_proof_index = self
            .proof_tree_state
            .injest_single_leaf_proof(InputLeafProof {
                leaf_circuit_type: CFC_LEAF_TYPE,
                fingerprint: fn_circuit_fingerprint,
                verifier_data: fn_circuit_verifier_data,
                proof: cfc_proof,
            })
            .await;
        let historical_root_proof = match self
            .proof_tree_state
            .find_zero_hash_proof_for_historical_root(cfc_proof_input.session_proof_tree_root)
            .await
        {
            Some(mp) => mp,
            None => anyhow::bail!("error finding historical root proof in proof_tree_state"),
        };
        let checkpoint_state = self.get_checkpoint_state();
        let last_tx_rec = self.lps.transaction_records.last().unwrap();
        let user_contract_tree_update_proof = last_tx_rec.user_contract_tree_update_proof.clone();
        let deferred_tx_pivot_index = debt_item.tree_index;
        let inline_tx_pivot_index = self.lps.get_inline_tx_debt_latest_index();
        let deferred_tx_debt_pivot_proof = self.lps.get_deferred_tx_tree_leaf(deferred_tx_pivot_index)?;
        let inline_tx_debt_pivot_proof = self.lps.get_inline_tx_tree_leaf(inline_tx_pivot_index)?;
        let proof_tree_inclusion_proof = self.proof_tree_state.get_leaf_merkle_proof(cfc_proof_index).await;
        let new_step_known_proof_tree_root = proof_tree_inclusion_proof.root;
        let verify_cfc_proof_input = AttestTreeAwareProofInTreeInput {
            fingerprint: fn_circuit_fingerprint,
            inner_public_inputs_hash: cfc_proof_input.tx_input_ctx.qfhash::<H>(),
            historical_root_proof,
            inclusion_proof: proof_tree_inclusion_proof,
        };

        // if !verify_cfc_proof_input.verify::<H>() {
        //     anyhow::bail!("AttestTreeAwareProofInTreeInput verification failed for
        // CFC deferred transaction step"); }

        let process_cfc_state_delta_input = UPSCFCStandardStateDeltaInput {
            cfc_transaction_input_context: cfc_proof_input.tx_input_ctx,
            user_contract_tree_update_proof,
            deferred_tx_debt_pivot_proof: deferred_tx_debt_pivot_proof.clone(),
            inline_tx_debt_pivot_proof: inline_tx_debt_pivot_proof.clone(),
        };
        let cfc_inclusion_proof = self
            .lps
            .get_contract_function_inclusion_proof(deferred_tx.contract_id.to_canonical_u64() as u32, fn_id.try_into().unwrap())
            .await?;
        let ups_cfc_standard_input = UPSVerifyCFCStandardStepInput {
            checkpoint_state,
            verify_cfc_proof_input,
            cfc_inclusion_proof,
            process_cfc_state_delta_input: process_cfc_state_delta_input.clone(),
        };
        let verify_previous_ups_step = self.get_verify_previous_ups_step_proof().await?;
        let deferred_input = UPSCFCDeferredTransactionCircuitInput {
            verify_previous_ups_step,
            deferred_tx_cfc_step: UPSVerifyPopDeferredTxStepInput {
                standard_cfc_verify_input: ups_cfc_standard_input,
                ups_pop_deferred_tx_proof: debt_removal_proof,
            },
        };
        let ups_proof = circuit_mgr.prove_ups_cfc_deferred_tx(&deferred_input).await?;
        self.last_ups_step_proof_info.circuit_id = LocalCircuitType::UPSCFCDeferred.into();
        let new_step_user_leaf = PsyUserLeaf {
            public_key: self.current_ups_header.current_state.user_leaf.public_key,
            user_state_tree_root: process_cfc_state_delta_input.user_contract_tree_update_proof.new_root,
            balance: process_cfc_state_delta_input
                .cfc_transaction_input_context
                .transaction_call_start_ctx
                .start_user_balance
                + process_cfc_state_delta_input
                    .cfc_transaction_input_context
                    .transaction_end_ctx
                    .total_balance_spent,
            event_index: process_cfc_state_delta_input
                .cfc_transaction_input_context
                .transaction_call_start_ctx
                .start_user_event_index
                + process_cfc_state_delta_input
                    .cfc_transaction_input_context
                    .transaction_end_ctx
                    .total_events_emitted,
            nonce: self.current_ups_header.current_state.user_leaf.nonce,
            last_checkpoint_id: self.current_ups_header.current_state.user_leaf.last_checkpoint_id,
            user_id: self.current_ups_header.current_state.user_leaf.user_id,
        };
        let tx_log_item = DPNProvingSessionSimpleMethodCall {
            caller_contract_id: deferred_tx.caller_contract_id,
            contract_id: deferred_tx.contract_id,
            method_id: deferred_tx.method_id,
            inputs: deferred_tx.inputs.clone(),
        };
        let tx_log_item_hash = tx_log_item.qfhash::<H>();
        let new_step_tx_hash_stack = H::q_two_to_one(self.current_ups_header.current_state.tx_hash_stack, tx_log_item_hash);
        let new_step_tx_count = self.current_ups_header.current_state.tx_count + F::ONE;
        let new_step_current_state = UserProvingSessionCurrentState {
            user_leaf: new_step_user_leaf,
            deferred_tx_debt_tree_root: deferred_tx_debt_pivot_proof.root,
            inline_tx_debt_tree_root: inline_tx_debt_pivot_proof.root,
            tx_hash_stack: new_step_tx_hash_stack,
            tx_count: new_step_tx_count,
        };
        let new_ups_header = UserProvingSessionHeader {
            ups_step_circuit_whitelist_root: self.current_ups_header.ups_step_circuit_whitelist_root,
            session_start_context: self.current_ups_header.session_start_context.clone(),
            current_state: new_step_current_state,
        };
        let ups_step_proof_tree_index = self
            .proof_tree_state
            .injest_single_leaf_proof(InputLeafProof {
                leaf_circuit_type: UPS_STEP_LEAF_TYPE,
                fingerprint: circuit_mgr.ups_cfc_deferred_tx_circuit_fingerprint().await?,
                verifier_data: circuit_mgr.ups_cfc_deferred_tx_circuit_verifier_config().await?,
                proof: ups_proof,
            })
            .await;
        self.last_ups_step_proof_info = TreeAwareTreeProofRecord {
            inner_public_inputs_hash: new_ups_header.qfhash::<H>(),
            circuit_id: LocalCircuitType::UPSCFCDeferred.into(),
            known_proof_tree_root: new_step_known_proof_tree_root,
            proof_tree_index: ups_step_proof_tree_index,
        };
        self.previous_ups_header = self.current_ups_header.clone();
        self.current_ups_header = new_ups_header;
        self.tx_log.push(tx_log_item);
        Ok(())
    }

    pub async fn get_api_input(&mut self) -> anyhow::Result<SubmitUserEndCapNonProofInput<F>> {
        // TODO
        // let checkpoint_id =
        // self.current_ups_header.session_start_context.checkpoint_id;
        let checkpoint_id = self.current_ups_header.current_state.user_leaf.last_checkpoint_id;

        let updates = self.get_user_session_update_history().await?;

        let core = SubmitUserEndCapNonProofCoreInput {
            checkpoint_id,
            stats: GUTAStats {
                fees_collected: F::from_canonical_u64(GUTA_FEE),
                user_ops_processed: F::from_noncanonical_u64(1),
                total_transactions: self.current_ups_header.current_state.tx_count,
                slots_modified: F::from_canonical_u32(updates.total_slots_modified),
            },
            state_transition: UPSEndCapResultCompact {
                start_user_leaf_hash: self.current_ups_header.session_start_context.start_session_user_leaf.qfhash::<H>(),
                end_user_leaf_hash: self.current_ups_header.current_state.user_leaf.qfhash::<H>(),
                checkpoint_tree_root_hash: self.current_ups_header.session_start_context.checkpoint_tree_root,
                user_id: self.current_ups_header.session_start_context.start_session_user_leaf.user_id,
            },
            new_user_leaf: self.current_ups_header.current_state.user_leaf,
        };
        let contract_state_updates = updates.contract_updates;

        Ok(SubmitUserEndCapNonProofInput {
            core,
            contract_state_updates,
        })
    }

    pub async fn get_user_session_update_history(&mut self) -> anyhow::Result<PsyUserSessionUpdateHistory<F>> {
        let (contract_updates, total_slots_modified) = self.lps.get_all_state_updates().await?;

        Ok(PsyUserSessionUpdateHistory {
            start_user_leaf: self.current_ups_header.session_start_context.start_session_user_leaf,
            end_user_leaf: self.current_ups_header.current_state.user_leaf,
            total_slots_modified,
            contract_updates,
        })
    }
}
