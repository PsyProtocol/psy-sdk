use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField, types::{Field, PrimeField64}},
    hash::hash_types::{HashOut, RichField},
    plonk::config::{AlgebraicHasher, GenericConfig},
};
use qed_common_circuit::treeprover::qrecursion::standard::manager::portable::core::PortableQTreeRecursionManager;
use qed_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use qed_core::{config::network_constants::{DEFERRED_TRANSACTION_TREE_HEIGHT, INLINE_TRANSACTION_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT}, data::qhashout::QHashOut, ups::circuits::LocalCircuitType, utils::debug_timer::DebugTimer};
use qed_crypto::{common::witnesses::qrecursion::{header::AttestTreeAwareProofInTreeInput, proof_data::{InputLeafProof, TreeAwareTreeProofRecord}}, hash::traits::{hasher::{FieldQHasher, MerkleZeroHasher}, qhashable::QFieldHashable}};
use qed_data::{
    dpn::proving_session::{DPNProvingSessionSimpleMethodCall, QEDLocalTransactionRecord}, qdata::{checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDCheckpointLeafCompact, QEDCheckpointLeafCompactWithStateRoots}, user::QEDUserLeaf}, ups::{start_step::UPSStartStepInput, ups_cfc_standard_step::UPSCFCStandardTransactionCircuitInput, ups_context_input::{UserProvingSessionCurrentState, UserProvingSessionHeader}, ups_standard_cfc_input::{UPSCFCStandardStateDeltaInput, UPSVerifyCFCStandardStepInput}, verify_previous_ups_step::VerifyPreviousUPSStepProofInProofTreeInput}
};
use qed_exec::vm::{cfc_input::DapenContractFunctionCircuitInput, exec::QEDEvalSessionResult};
use qed_store::{
    controllers::local::{proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore}, store::imm::{cmd::{QSRCmdGetCheckpointLeafData, QSRMerkleCmd, QSRMerkleCmdGetCheckpointTreeMerkleProof, QSRMerkleCmdGetUserTreeMerkleProof}, cmd_processor::{QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}}
};
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

use crate::dpn::circuits::cfc::DapenContractFunctionCircuit;

use super::circuit_manager::core::QEDUPSStepCircuitManager;

const UPS_STEP_LEAF_TYPE: u64 = 1;
const CFC_LEAF_TYPE: u64 = 2;

#[derive(Clone, Debug)]
pub struct UserProvingSessionManager<
    F: RichField + Extendable<D>,
    H: MerkleZeroHasher<QHashOut<F>> + MerkleZeroHasher<HashOut<F>> + AlgebraicHasher<F>,
    R: QEDReadCommandProcessorSync<F>,
    C: GenericConfig<D, F = F, Hasher = H>,
    const D: usize,
> {
    lps: QEDLocalProvingSessionStore<F, R>,
    circuit_info: SessionCircuitInfoStore<F>,
    proof_tree_state: PortableQTreeRecursionManager<C, D>,
    current_ups_header: UserProvingSessionHeader<F>,
    current_checkpoint_leaf: QEDCheckpointLeaf<F>,
    current_global_state_roots: QEDCheckpointGlobalStateRoots<F>,
    last_ups_step_proof_info: TreeAwareTreeProofRecord<F>,

    
    tx_log: Vec<DPNProvingSessionSimpleMethodCall<F>>,
}


type F = GoldilocksField;
const D: usize = 2;
impl<
        H: MerkleZeroHasher<QHashOut<F>> + MerkleZeroHasher<HashOut<F>> + AlgebraicHasher<F> + FieldQHasher<F>,
        R: QEDReadCommandProcessorSync<F>,
        C: GenericConfig<D, F = F, Hasher = H>,
    > UserProvingSessionManager<F, H, R, C, D>
{
    pub fn get_checkpoint_state(&self) -> QEDCheckpointLeafCompactWithStateRoots<F> {
        QEDCheckpointLeafCompactWithStateRoots {
            checkpoint_leaf: QEDCheckpointLeafCompact {
                global_chain_root: self.current_checkpoint_leaf.global_chain_root,
                stats_hash: self.current_checkpoint_leaf.stats.qfhash::<H>(),
            },
            global_state_roots: self.current_global_state_roots,
        }
    }
    pub fn new(
        mut lps: QEDLocalProvingSessionStore<F, R>,
        circuit_info: SessionCircuitInfoStore<F>,
        ups_step_circuit_whitelist_root: QHashOut<F>,
    ) -> anyhow::Result<Self> {
        let proof_tree_state = PortableQTreeRecursionManager::<C, D>::new(
            UPS_SESSION_PROOF_TREE_HEIGHT as usize
        );
        let session_start_context = lps.get_ups_start_ctx()?;
        
        let mut new_user=  session_start_context.start_session_user_leaf.clone();

        //let l2_bstate = lps.cmd_store.resolve_get_latest_l2_block_state_mut()?;

        let latest_checkpoint_id_u64 = lps.get_current_start_checkpoint_id_u64();
        let latest_checkpoint_id_f = lps.get_current_start_checkpoint_id();
        new_user.last_checkpoint_id = latest_checkpoint_id_f;
        println!("checkpoint_id: {}",latest_checkpoint_id_u64);

        let current_checkpoint_leaf = lps
            .cmd_store
            .resolve_get_checkpoint_leaf_mut(&QSRCmdGetCheckpointLeafData { checkpoint_id: latest_checkpoint_id_u64 })?;

        let current_global_state_roots = lps.get_global_state_tree_roots(latest_checkpoint_id_u64)?;




        let current_state = UserProvingSessionCurrentState{
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
            current_ups_header,
            current_checkpoint_leaf,
            current_global_state_roots,
            last_ups_step_proof_info: TreeAwareTreeProofRecord::default(),
            circuit_info,
            tx_log: vec![],
        })
    }

    pub fn get_ups_start_witness(
        &mut self,
    ) -> anyhow::Result<UPSStartStepInput<F>> {
        
        let checkpoint_tree_proof= self.lps.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetCheckpointTreeMerkleProof(QSRMerkleCmdGetCheckpointTreeMerkleProof{
            checkpoint_id: self.lps.get_current_write_checkpoint_id_u64(),
            leaf_checkpoint_id: self.lps.get_current_start_checkpoint_id_u64(),
        }))?;

        let user_tree_proof =
            self.lps.cmd_store
                .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserTreeMerkleProof(
                    QSRMerkleCmdGetUserTreeMerkleProof {
                        checkpoint_id: self.lps.get_current_write_checkpoint_id_u64(),
                        user_id: self.lps.get_current_user_id_64(),
                    },
                ))?;


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

    pub fn prove_ups_start(&mut self, circuit_mgr: &QEDUPSStepCircuitManager<C, D>) -> anyhow::Result<()> {
        let mut timer = DebugTimer::new("prove_ups_start");
        timer.lap("start");
        let input = self.get_ups_start_witness()?;
        println!("witness:\n{:?}",input);
        println!("\n\n\nwitness json:\n{}\n\n\n\n\n\n",serde_json::to_string_pretty(&input).unwrap());
        
        timer.lap("gen_witness");
        
        let proof = circuit_mgr.ups_start.prove_base(&input)?;

        
        timer.lap("prove_ups_start");
        let known_proof_tree_root = self.proof_tree_state.get_proof_tree_root();
        let inner_public_inputs_hash = input.ups_header.qfhash::<H>();

        let last_ups_step_proof_index = self.proof_tree_state.injest_single_leaf_proof(InputLeafProof{
            leaf_circuit_type: UPS_STEP_LEAF_TYPE,
            fingerprint: circuit_mgr.ups_start.get_fingerprint(),
            verifier_data: circuit_mgr.ups_start.get_verifier_config_ref().to_owned(),
            proof,
        });
        self.last_ups_step_proof_info = TreeAwareTreeProofRecord {
            circuit_id: LocalCircuitType::UPSStart.into(),
            inner_public_inputs_hash,
            known_proof_tree_root,
            proof_tree_index: last_ups_step_proof_index,
        };
        self.current_ups_header = input.ups_header;
        timer.lap("injest_single_leaf_proof");

        Ok(())
    }
    /* 
    pub fn get_verify_previous_ups_step_proof_input_std_cfc(&self, ups_circuit_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>) -> anyhow::Result<VerifyPreviousUPSStepProofInProofTreeInput<F>> {
        let x = VerifyPreviousUPSStepProofInProofTreeInput {
            proof_attestation_witness: todo!(),
            previous_step_header: self.current_ups_header.clone(),
            ups_circuit_whitelist_merkle_proof,
        };
        Ok(x)
    }*/
    pub fn get_verify_previous_ups_step_proof(&mut self) -> anyhow::Result<VerifyPreviousUPSStepProofInProofTreeInput<F>> {
        let previous_step_header = self.current_ups_header.clone();
        let ups_circuit_whitelist_merkle_proof = self.circuit_info.get_whitelist_merkle_proof(
            self.last_ups_step_proof_info.circuit_id
        )?.to_owned();
        let historical_root_proof = match self.proof_tree_state.find_zero_hash_proof_for_historical_root(self.last_ups_step_proof_info.known_proof_tree_root) {
            Some(p) => p,
            None => anyhow::bail!("could not find historical root proof for root {:?}",self.last_ups_step_proof_info.known_proof_tree_root),
        };
        let inclusion_proof = self.proof_tree_state.get_leaf_merkle_proof(self.last_ups_step_proof_info.proof_tree_index);


        let proof_attestation_witness = AttestTreeAwareProofInTreeInput {
            fingerprint: ups_circuit_whitelist_merkle_proof.value,
            inner_public_inputs_hash: self.last_ups_step_proof_info.inner_public_inputs_hash,
            historical_root_proof,
            inclusion_proof,
        };
        Ok(VerifyPreviousUPSStepProofInProofTreeInput{
            proof_attestation_witness,
            previous_step_header,
            ups_circuit_whitelist_merkle_proof,
        })

    }
    pub fn prove_contract_call(
        &mut self,
        circuit_mgr: &QEDUPSStepCircuitManager<C, D>,
        contract_id: F,
        fn_id: u32,
        fn_circuit: &DapenContractFunctionCircuit<C, D>,
        fn_circuit_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<()> {
        //let last_session_header = self.current_ups_header.clone();
        //let ups_standard_whitelist_merkle_proof = circuit_mgr.ups_cfc_standard_tx_whitelist_proof.clone();
        let deferred_tx_pivot_index = self.lps.get_deferred_tx_debt_latest_index();
        let inline_tx_pivot_index = self.lps.get_inline_tx_debt_latest_index();

        
        let tx_log_item = DPNProvingSessionSimpleMethodCall {
            contract_id,
            method_id: F::from_canonical_u32(fn_circuit_def.method_id),
            inputs: inputs.clone(),
        };

        let cfc_proof_input = self.exec_contract_call(contract_id, fn_circuit_def, inputs)?;

        
        let cfc_proof = fn_circuit.prove_base(&cfc_proof_input)?;

        let cfc_proof_index = self.proof_tree_state.injest_single_leaf_proof(InputLeafProof{
            leaf_circuit_type: CFC_LEAF_TYPE,
            fingerprint: fn_circuit.get_fingerprint(),
            verifier_data: fn_circuit.get_verifier_config_ref().to_owned(),
            proof: cfc_proof,
        });
        let cfc_inclusion_proof = self.lps.get_contract_function_inclusion_proof(
            contract_id.to_canonical_u64() as u32,
            fn_id,
        )?;
        println!("cfc_proof_input.session_proof_tree_root: {:?}",&cfc_proof_input.session_proof_tree_root);
        let historical_root_proof =  match self.proof_tree_state.find_zero_hash_proof_for_historical_root(cfc_proof_input.session_proof_tree_root) {
            Some(mp) => mp,
            None => anyhow::bail!("error finding historical root proof in proof_tree_state"),
        };

        let checkpoint_state = self.get_checkpoint_state();
        let last_tx_rec: &QEDLocalTransactionRecord<GoldilocksField> = self.lps.transaction_records.last().unwrap();
        let user_contract_tree_update_proof = last_tx_rec.user_contract_tree_update_proof.clone();
        let deferred_tx_debt_pivot_proof = self.lps.get_deferred_tx_tree_leaf(deferred_tx_pivot_index)?;
        let inline_tx_debt_pivot_proof = self.lps.get_inline_tx_tree_leaf(inline_tx_pivot_index)?;
        let new_step_deferred_tx_debt_tree_root = deferred_tx_debt_pivot_proof.root;
        let new_step_inline_tx_debt_tree_root = inline_tx_debt_pivot_proof.root;
        
        let proof_tree_inclusion_proof = self.proof_tree_state.get_leaf_merkle_proof(cfc_proof_index);
        let new_step_known_proof_tree_root = proof_tree_inclusion_proof.root;
        let verify_cfc_proof_input = AttestTreeAwareProofInTreeInput {
            fingerprint: fn_circuit.get_fingerprint(),
            inner_public_inputs_hash: cfc_proof_input.tx_input_ctx.qfhash::<H>(),
            historical_root_proof,
            inclusion_proof: proof_tree_inclusion_proof,
        };
        let process_cfc_state_delta_input = UPSCFCStandardStateDeltaInput {
            cfc_transaction_input_context: cfc_proof_input.tx_input_ctx,
            user_contract_tree_update_proof,
            deferred_tx_debt_pivot_proof,
            inline_tx_debt_pivot_proof,
        };



        let new_step_user_leaf = QEDUserLeaf {
            public_key: self.current_ups_header.current_state.user_leaf.public_key,
            user_state_tree_root: process_cfc_state_delta_input.user_contract_tree_update_proof.new_root,
            balance: process_cfc_state_delta_input.cfc_transaction_input_context.transaction_call_start_ctx.start_user_balance+process_cfc_state_delta_input.cfc_transaction_input_context.transaction_end_ctx.total_balance_spent,
            event_index: process_cfc_state_delta_input.cfc_transaction_input_context.transaction_call_start_ctx.start_user_event_index+process_cfc_state_delta_input.cfc_transaction_input_context.transaction_end_ctx.total_events_emitted,
            nonce: self.current_ups_header.current_state.user_leaf.nonce,
            last_checkpoint_id: self.current_ups_header.current_state.user_leaf.last_checkpoint_id,
            user_id:  self.current_ups_header.current_state.user_leaf.user_id,
        };

        let tx_log_item_hash = tx_log_item.qfhash::<H>();
        let new_step_tx_hash_stack = H::q_two_to_one(
            self.current_ups_header.current_state.tx_hash_stack,
            tx_log_item_hash,
        );
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
        let verify_previous_ups_step = self.get_verify_previous_ups_step_proof()?;

        let circuit_input = UPSCFCStandardTransactionCircuitInput {
            verify_previous_ups_step,
            standard_cfc_step: ups_cfc_standard_input,
        };

        let ups_proof = circuit_mgr.ups_cfc_standard_tx.prove_base(&circuit_input)?;
        self.last_ups_step_proof_info.circuit_id = LocalCircuitType::UPSCFCStandard.into();
        
        let new_ups_header = UserProvingSessionHeader {
            ups_step_circuit_whitelist_root: self.current_ups_header.ups_step_circuit_whitelist_root,
            session_start_context: self.current_ups_header.session_start_context.clone(),
            current_state: new_step_current_state,
        };

        let ups_step_proof_tree_index = self.proof_tree_state.injest_single_leaf_proof(InputLeafProof{
            leaf_circuit_type: UPS_STEP_LEAF_TYPE,
            fingerprint: circuit_mgr.ups_cfc_standard_tx.get_fingerprint(),
            verifier_data: circuit_mgr.ups_cfc_standard_tx.get_verifier_config_ref().to_owned(),
            proof: ups_proof,
        });
        self.last_ups_step_proof_info = TreeAwareTreeProofRecord{
            inner_public_inputs_hash: new_ups_header.qfhash::<H>(),
            circuit_id: LocalCircuitType::UPSCFCStandard.into(),
            known_proof_tree_root: new_step_known_proof_tree_root,
            proof_tree_index: ups_step_proof_tree_index,
        };
        self.current_ups_header = new_ups_header;
        self.tx_log.push(tx_log_item);

        

        Ok(())
        





    }
    pub fn exec_contract_call(
        &mut self,
        contract_id: F,
        fn_circuit_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<F>> {
        self.lps.set_proof_tree_root(
            self.proof_tree_state.get_proof_tree_root()
        );
        QEDEvalSessionResult::new()
            .exec_contract_call( 
                &mut self.lps,
                contract_id,
                fn_circuit_def,
                inputs
            )
    }
}
