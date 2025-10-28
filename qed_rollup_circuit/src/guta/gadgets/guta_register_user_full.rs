use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::{target::{BoolTarget, Target}, witness::Witness}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_common_circuit::{builder::comparison::CircuitBuilderComparison, hash::merkle::gadgets::merkle_proof::MerkleProofGadget, treeprover::subtree::gadgets::subtree_core::SubTreeNodeStateTransitionGadget};
use qed_core::data::qhashout::QHashOut;
use psy_crypto::{common::user_id::circuit_user_registration_tree_index_bits_to_user_id, hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore}};


use super::guta_register_user_core::GUTARegisterUserCoreGadget;




#[derive(Clone, Debug)]
pub struct GUTARegisterUserFullGadget {
    pub user_registration_tree_merkle_proof: MerkleProofGadget,
    pub register_user_core_gadget: GUTARegisterUserCoreGadget,


    // computed
    pub user_registration_tree_root: HashOutTarget,
    pub old_global_user_tree_root: HashOutTarget,
    pub new_global_user_tree_root: HashOutTarget,

    pub global_user_tree_proof_height: Target,
}

impl GUTARegisterUserFullGadget {
    pub fn add_virtual_to<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        global_user_tree_realm_height: usize,
        global_user_tree_height: usize,
        default_user_state_tree_root: QHashOut<F>,
        input_height_target: Option<Target>,
    ) -> Self {

        let (
            user_registration_tree_merkle_proof,
            user_registration_tree_index_bits,
        ) = MerkleProofGadget::add_virtual_to_get_index_bits::<H,F,D>(
            builder,
            global_user_tree_height,
        );

        let expected_user_id = circuit_user_registration_tree_index_bits_to_user_id::<H,F,D>(
            builder,
            user_registration_tree_merkle_proof.index,
            &user_registration_tree_index_bits,
            global_user_tree_height
        );

        let public_key = user_registration_tree_merkle_proof.value;

        builder.assert_non_zero_hash(public_key);


        let register_user_core_gadget = GUTARegisterUserCoreGadget::add_virtual_to_with_public_key::<H,F,D>(
            builder,
            global_user_tree_realm_height,
            global_user_tree_height,
            default_user_state_tree_root,
            input_height_target,
            public_key,
        );

        builder.connect(
            register_user_core_gadget.user_id,
            expected_user_id,
        );


        let user_registration_tree_root = user_registration_tree_merkle_proof.root;
        let old_global_user_tree_root = register_user_core_gadget.global_user_tree_update_proof.old_root;
        let new_global_user_tree_root = register_user_core_gadget.global_user_tree_update_proof.new_root;

        let global_user_tree_proof_height = register_user_core_gadget.global_user_tree_update_proof.height;

        Self {
            user_registration_tree_merkle_proof,
            register_user_core_gadget,
            user_registration_tree_root,
            old_global_user_tree_root,
            new_global_user_tree_root,
            global_user_tree_proof_height,
        }
    }

    pub fn get_state_transition<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> SubTreeNodeStateTransitionGadget {
        self.register_user_core_gadget.get_state_transition(builder)
    }

    pub fn set_witness_params<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        user_registration_tree_merkle_proof: &MerkleProofCore<QHashOut<F>>,
        global_user_tree_update_proof: &DeltaMerkleProofCore<QHashOut<F>>,
    ) -> anyhow::Result<()> {
        self.user_registration_tree_merkle_proof.set_witness_core_proof_q_generic(
            witness,
            user_registration_tree_merkle_proof,
        )?;
        self.register_user_core_gadget.set_witness_params_no_public_key(
            witness,
            global_user_tree_update_proof,
        )
    }

}
