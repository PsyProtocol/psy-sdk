use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::{target::Target, witness::Witness},
    plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher},
};
use qed_common_circuit::treeprover::qrecursion::standard::gadgets::attest_proof_in_tree::AttestProofInTreeGadget
;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::{common::witnesses::qrecursion::header::AttestProofInTreeInput, hash::traits::hasher::MerkleZeroHasher};
use qed_data::ups::{ups_end_cap::UPSEndCapFromProofTreeGadgetInput, verify_previous_ups_step::VerifyPreviousUPSStepProofInProofTreeInput};


use super::{ups_end_cap::UPSEndCapCoreGadget, verify_previous_ups_step::VerifyPreviousUPSStepProofInProofTreeGadget};




#[derive(Clone, Debug)]
pub struct UPSEndCapFromProofTreeGadget {

    // start require witness
    pub verify_previous_ups_step_gadget: VerifyPreviousUPSStepProofInProofTreeGadget,
    pub verify_zk_signature_proof_gadget: AttestProofInTreeGadget,
    pub user_public_key_param: HashOutTarget,
    pub nonce: Target,


    // start computed
    pub end_cap_core_gadget: UPSEndCapCoreGadget,
    pub current_proof_tree_root: HashOutTarget,

    
}
impl UPSEndCapFromProofTreeGadget {
    pub fn add_virtual_to<H: AlgebraicHasher<F>+MerkleZeroHasher<HashOut<F>>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        ups_session_proof_tree_height: usize,
        ups_circuit_whitelist_tree_height: usize,
        network_magic: u64,
    ) -> Self {

        let user_public_key_param = builder.add_virtual_hash();
        let nonce = builder.add_virtual_target();

        let verify_previous_ups_step_gadget = VerifyPreviousUPSStepProofInProofTreeGadget::add_virtual_to::<H,F,D>(
             builder,
            ups_session_proof_tree_height,
            ups_circuit_whitelist_tree_height,
        );

        let current_proof_tree_root = verify_previous_ups_step_gadget.current_proof_tree_root;

        let verify_zk_signature_proof_gadget = AttestProofInTreeGadget::add_virtual_to::<H,F,D>(
            builder,
            ups_session_proof_tree_height,
        );

        // ensure our zk signature is in the same proof tree as our previous ups step
        builder.connect_hashes(
            verify_zk_signature_proof_gadget.attested_proof_tree_root,
            current_proof_tree_root,
        );



        let end_cap_core_gadget = UPSEndCapCoreGadget::enforce_signature_constraints::<H,F,D>(
            builder, 
            &verify_previous_ups_step_gadget.previous_step_header_gadget, 
            verify_zk_signature_proof_gadget.public_inputs_hash, 
            verify_zk_signature_proof_gadget.fingerprint, 
            user_public_key_param, 
            nonce, 
            network_magic
        );

        Self {
            verify_previous_ups_step_gadget,
            verify_zk_signature_proof_gadget,
            user_public_key_param,
            nonce,

            end_cap_core_gadget,
            current_proof_tree_root,
        }
    }
    pub fn set_witness_params<F: RichField>(
        &self,
        witness: &mut impl Witness<F>,
        verify_previous_ups_step_input: &VerifyPreviousUPSStepProofInProofTreeInput<F>,
        verify_zk_signature_proof_input: &AttestProofInTreeInput<F>,
        user_public_key_param: QHashOut<F>,
        nonce: F,
    ) {

        witness.set_hash_target(
            self.user_public_key_param,
            user_public_key_param.0,
        );
        witness.set_target(
            self.nonce,
            nonce,
        );

        self.verify_previous_ups_step_gadget.set_witness(
            witness,
            verify_previous_ups_step_input,
        );
        self.verify_zk_signature_proof_gadget.set_witness(
            witness,
            verify_zk_signature_proof_input,
        );
    }
    pub fn set_witness<F: RichField>(
        &self,
        witness: &mut impl Witness<F>,
        target: &UPSEndCapFromProofTreeGadgetInput<F>,
    ) {
        self.set_witness_params(
            witness, 
            &target.verify_previous_ups_step_input,
            &target.verify_zk_signature_proof_input, 
            target.user_public_key_param,
            target.nonce
        );

    }

}