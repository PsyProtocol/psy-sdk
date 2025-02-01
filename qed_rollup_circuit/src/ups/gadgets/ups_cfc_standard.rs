use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::witness::Witness,
    plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher},
};
use qed_common_circuit::{
    builder::hash::core::CircuitBuilderHashCore, hash::merkle::gadgets::merkle_proof::MerkleProofGadget, traits::{CreatableTarget, CreatableWithHasherTarget, WitnessValueFor}, treeprover::qrecursion::standard::gadgets::attest_tree_aware_proof_in_tree::AttestTreeAwareProofInTreeGadget
};
use qed_core::{
    config::network_constants::{CHECKPOINT_TREE_HEIGHT, DEFERRED_TRANSACTION_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, INLINE_TRANSACTION_TREE_HEIGHT},
    data::qhashout::QHashOut,
};
use qed_crypto::hash::traits::hasher::{iterate_merkle_hasher_alg, MerkleZeroHasher};
use qed_data::ups::{start_step::UPSStartStepInput, ups_standard_cfc_input::UPSVerifyCFCStandardStepInput};

use crate::gadgets::qdata::{
    cfc_context_input::UPSInspectDapenCFCUserTransactionInputContextGadget, checkpoint::QEDCheckpointLeafGadget, checkpoint_compact_with_state::QEDCheckpointLeafCompactWithStateRootsGadget, checkpoint_state_roots::QEDCheckpointGlobalStateRootsGadget, contract_inclusion::QEDContractFunctionInclusionProofGadget, ups_context_input::{UserProvingSessionCurrentStateGadget, UserProvingSessionHeaderGadget}
};

use super::{ups_cfc_verify_inclusion::UPSVerifyCFCProofExistsAndValidGadget, ups_standard_cfc_state_delta::UPSCFCStandardStateDeltaGadget, verify_previous_ups_step::VerifyPreviousUPSStepProofInProofTreeGadget};

#[derive(Clone, Debug)]
pub struct UPSVerifyCFCStandardStepGadget {

    // start require witness
    pub verify_cfc_exists_and_valid_gadget: UPSVerifyCFCProofExistsAndValidGadget,    
    pub process_cfc_state_delta_gadget: UPSCFCStandardStateDeltaGadget,

    // start computed
    pub new_header_gadget: UserProvingSessionHeaderGadget,
    
}
impl UPSVerifyCFCStandardStepGadget {
    fn add_virtual_to<H: AlgebraicHasher<F> + MerkleZeroHasher<HashOut<F>>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        previous_step_header_gadget: &UserProvingSessionHeaderGadget,
        current_proof_tree_root: HashOutTarget,
        q_recursion_tree_height: usize,
    ) -> Self {
        // start require witness
        let verify_cfc_exists_and_valid_gadget = UPSVerifyCFCProofExistsAndValidGadget::add_virtual_to::<H,F,D>(
            builder,
            q_recursion_tree_height,
        );
            
        let (process_cfc_state_delta_gadget, new_header_gadget) = UPSCFCStandardStateDeltaGadget::add_virtual_to::<H,F,D>(builder, previous_step_header_gadget);


        // constrain verify with previous
        // ensure the cfc proof verifier gadget is using the correct proof tree root (ie. so we know this proof actually exists if UPS is later to believed)
        builder.connect_hashes(
            verify_cfc_exists_and_valid_gadget.attested_proof_tree_root, 
            current_proof_tree_root,
        );

        // ensure the cfc proof verifier is working with the correct checkpoint leaf hash (ie. so the contract function tree is correct) 
        builder.connect_hashes(
            verify_cfc_exists_and_valid_gadget.checkpoint_leaf_hash, 
            previous_step_header_gadget.session_start_context.checkpoint_leaf_hash,
        );

        // constraint process_cfc_state_delta
        // ensure that the verifier and state processor agree on contract id
        builder.connect(
            verify_cfc_exists_and_valid_gadget.cfc_contract_id,
            process_cfc_state_delta_gadget.cfc_contract_id,
        );

        // ensure that the verifier and state processor agree on method id
        builder.connect(
            verify_cfc_exists_and_valid_gadget.cfc_method_id,
            process_cfc_state_delta_gadget.cfc_method_id,
        );

        // ensure that the verifier and state processor agree on number of inputs
        builder.connect(
            verify_cfc_exists_and_valid_gadget.cfc_num_inputs,
            process_cfc_state_delta_gadget.cfc_num_inputs,
        );

        // ensure that the verifier and state processor agree on number of outputs
        builder.connect(
            verify_cfc_exists_and_valid_gadget.cfc_num_outputs,
            process_cfc_state_delta_gadget.cfc_num_outputs,
        );

        // ensure that the verifier and state processor agree on the inner_public_inputs_hash
        builder.connect_hashes(
            verify_cfc_exists_and_valid_gadget.cfc_inner_public_inputs_hash,
            process_cfc_state_delta_gadget.cfc_inner_public_inputs_hash,
        );



        
        Self {
            verify_cfc_exists_and_valid_gadget,
            process_cfc_state_delta_gadget,
            new_header_gadget
        }
    }
    pub fn set_witness<F: RichField>(
        &self,
        witness: &mut impl Witness<F>,
        target: &UPSVerifyCFCStandardStepInput<F>,
    ) {
        self.verify_cfc_exists_and_valid_gadget.set_witness_params(
            witness, 
            &target.checkpoint_state,
            &target.verify_cfc_proof_input,
            &target.cfc_inclusion_proof
        );

        self.process_cfc_state_delta_gadget.set_witness(witness, &target.process_cfc_state_delta_input);
    }
}


impl<F: RichField> WitnessValueFor<UPSVerifyCFCStandardStepGadget, F, true> for UPSStartStepInput<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &UPSVerifyCFCStandardStepGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<UPSStartStepGadget, F, false> for UPSStartStepInput<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &UPSStartStepGadget) {
        target.set_witness(witness, self);
    }
}
