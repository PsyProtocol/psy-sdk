use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::witness::Witness,
    plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher},
};
use qed_common_circuit::{hash::merkle::gadgets::delta_merkle_proof::DeltaMerkleProofGadget, 
    traits::WitnessValueFor}
;
use qed_core::config::network_constants::DEFERRED_TRANSACTION_TREE_HEIGHT;
use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
use qed_data::ups::ups_standard_cfc_input::UPSVerifyPopDeferredTxStepInput;

use crate::gadgets::{qdata::
    ups_context_input::UserProvingSessionHeaderGadget, stack_items::deferred_transaction::DeferredTransactionStackItemGadget}
;

use super::{correct_header_hashes::CorrectUPSHeaderHashesGadget, ups_cfc_standard::UPSVerifyCFCStandardStepGadget};


#[derive(Clone, Debug)]
pub struct UPSVerifyPopDeferredTxStepGadget {

    // start require witness
    pub standard_cfc_verify_gadget: UPSVerifyCFCStandardStepGadget,
    pub ups_pop_deferred_tx_proof: DeltaMerkleProofGadget,
    
}
impl UPSVerifyPopDeferredTxStepGadget {
    pub fn add_virtual_to<H: AlgebraicHasher<F> + MerkleZeroHasher<HashOut<F>>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        previous_step_header_gadget: &UserProvingSessionHeaderGadget,
        current_proof_tree_root: HashOutTarget,
        q_recursion_tree_height: usize,
    ) -> Self {
        let ups_pop_deferred_tx_proof = DeltaMerkleProofGadget::add_virtual_to::<H, F, D>(
            builder, 
            DEFERRED_TRANSACTION_TREE_HEIGHT as usize
        );

        let expected_deferred_tx_leaf_hash = ups_pop_deferred_tx_proof.new_value;

        // ensure ups_pop_deferred_tx_proof's old root is the same as previous_step_header_gadget's deferred_tx_debt_tree_root
        builder.connect_hashes(
            ups_pop_deferred_tx_proof.old_root,
            previous_step_header_gadget.current_state.deferred_tx_debt_tree_root,
        );

        // ensure that the ups_pop_deferred_tx_proof removes an element from the tree (set the leaf to zero)
        let zero_hash = builder.constant_hash(HashOut::ZERO);
        builder.connect_hashes(
            ups_pop_deferred_tx_proof.new_value,
            zero_hash,
        );


        let mut corrections  = CorrectUPSHeaderHashesGadget::from_previous_step(previous_step_header_gadget);

        // CREATE CORRECTION: modify the previous step's deferred_tx_debt_tree_root to be ups_pop_deferred_tx_proof.new_root
        // ie. start the deferred debt tree in the tx as it is AFTER removing remove the current transaction
        corrections.previous_step_deferred_tx_debt_tree_root = ups_pop_deferred_tx_proof.new_root;

        
    

        let standard_cfc_verify_gadget = UPSVerifyCFCStandardStepGadget::add_virtual_to_with_corrections::<H,F,D>(
            builder, 
            previous_step_header_gadget, 
            &corrections, 
            current_proof_tree_root, 
            q_recursion_tree_height
        );


        // compute the hash of the deferred transaction item for the transaction we just proved 
        let computed_deferred_tx_stack_item: DeferredTransactionStackItemGadget = DeferredTransactionStackItemGadget {
            call_data: standard_cfc_verify_gadget.process_cfc_state_delta_gadget.cfc_transaction_input_context.transaction_call_start_ctx.call_data,
        };
        let computed_deferred_tx_leaf_hash = computed_deferred_tx_stack_item.to_hash::<H,F,D>(builder);

        // ensure that the trasnaction we just proved matches/fulfills the deferred transaction debt item we removed from the tree
        builder.connect_hashes(
            expected_deferred_tx_leaf_hash,
            computed_deferred_tx_leaf_hash,
        );

        
        Self {
            standard_cfc_verify_gadget,
            ups_pop_deferred_tx_proof,
        }
    }
    pub fn set_witness<F: RichField>(
        &self,
        witness: &mut impl Witness<F>,
        target: &UPSVerifyPopDeferredTxStepInput<F>,
    ) {
        self.standard_cfc_verify_gadget.set_witness(
            witness, 
            &target.standard_cfc_verify_input
        );

        self.ups_pop_deferred_tx_proof.set_witness_core_proof_q(witness, &target.ups_pop_deferred_tx_proof);
    }
}


impl<F: RichField> WitnessValueFor<UPSVerifyPopDeferredTxStepGadget, F, true> for UPSVerifyPopDeferredTxStepInput<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &UPSVerifyPopDeferredTxStepGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<UPSVerifyPopDeferredTxStepGadget, F, false> for UPSVerifyPopDeferredTxStepInput<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &UPSVerifyPopDeferredTxStepGadget) {
        target.set_witness(witness, self);
    }
}
