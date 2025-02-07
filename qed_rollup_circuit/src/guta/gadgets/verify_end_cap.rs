use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::witness::Witness,
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget},
    },
};
use qed_common_circuit::{builder::{core::CircuitBuilderHelpersCore, hash::core::CircuitBuilderHashCore, verify::CircuitBuilderVerifyProofHelpers}, hash::merkle::gadgets::historical_root_merkle_proof::HistoricalRootMerkleProofGadget, treeprover::subtree::gadgets::subtree_core::SubTreeNodeStateTransitionGadget};
use qed_core::{config::network_constants::{CHECKPOINT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT}, data::qhashout::QHashOut};
use qed_crypto::hash::{merkle::core::MerkleProofCore, traits::hasher::MerkleZeroHasher};
use qed_data::{guta::stats::GUTAStats, qdata::ups_end_cap_result::UPSEndCapResultCompact};

use crate::ups::gadgets::ups_end_cap_result::UPSEndCapResultCompactGadget;

use super::{guta_header::GlobalUserTreeAggregatorHeaderGadget, guta_stats::GUTAStatsGadget, helpers::ToGUTAHeader};

#[derive(Clone, Debug)]
pub struct VerifyEndCapProofGadget<const D: usize> {
    // start targets requiring witness
    pub end_cap_result_gadget: UPSEndCapResultCompactGadget,
    pub guta_stats: GUTAStatsGadget,
    pub checkpoint_historical_merkle_proof: HistoricalRootMerkleProofGadget,
    pub verifier_data: VerifierCircuitTarget,
    pub proof_target: ProofWithPublicInputsTarget<D>,
    // end targets requiring witness
}

impl<const D: usize> VerifyEndCapProofGadget<D> {
    pub fn add_virtual_to<C: GenericConfig<D, F = F>, F: RichField + Extendable<D>>(
        builder: &mut CircuitBuilder<F, D>,
        proof_common_data: &CommonCircuitData<F, D>,
        verifier_data_cap_height: usize,
        known_end_cap_fingerprint_hash: HashOutTarget,
    ) -> Self
    where
        <C as GenericConfig<D>>::Hasher: MerkleZeroHasher<HashOut<F>> +AlgebraicHasher<F>,
    {
        let verifier_data = builder.add_virtual_verifier_data(verifier_data_cap_height);
        let proof_target = builder.add_virtual_proof_with_pis(proof_common_data);

        builder.verify_proof::<C>(&proof_target, &verifier_data, proof_common_data);

        let proof_fingerprint = builder.get_circuit_fingerprint::<C::Hasher>(&verifier_data);


        // ensure the proof has the correct fingerprint
        builder.connect_hashes(
            known_end_cap_fingerprint_hash,
            proof_fingerprint,
        );

        let checkpoint_historical_merkle_proof = HistoricalRootMerkleProofGadget::add_virtual_to_zero_gt::<C::Hasher, C::F, D>(builder, CHECKPOINT_TREE_HEIGHT as usize);

        let end_cap_result_gadget = UPSEndCapResultCompactGadget::add_virtual_to::<F, D>(builder);
        let guta_stats = GUTAStatsGadget::add_virtual_to::<F, D>(builder);


        // start: check child proof public inputs

        let state_transition_pi_hash = end_cap_result_gadget.to_hash::<C::Hasher, C::F, D>(builder);
        let guta_stats_pi_hash = guta_stats.to_hash::<C::Hasher, C::F, D>(builder);

        let expected_proof_public_inputs_hash = builder.hash_two_to_one::<C::Hasher>(state_transition_pi_hash, guta_stats_pi_hash);
        


        assert_eq!(
            proof_target.public_inputs.len(),
            4,
            "children proofs should have 4 public inputs"
        );
        let proof_public_input_hash = HashOutTarget {
            elements: [
                proof_target.public_inputs[0],
                proof_target.public_inputs[1],
                proof_target.public_inputs[2],
                proof_target.public_inputs[3],
            ],
        };

        // ensure the whitelist root and state transition is correct for the proof
        builder.connect_hashes(expected_proof_public_inputs_hash, proof_public_input_hash);
        // end: check child proof public inputs


        // ensure the checkpoint root being used by the user is a valid checkpoint root in the tree (in the past)
        builder.connect_hashes(
            checkpoint_historical_merkle_proof.historical_root, 
            end_cap_result_gadget.checkpoint_tree_root_hash,
        );

        Self {
            verifier_data,
            proof_target,
            end_cap_result_gadget,
            guta_stats,
            checkpoint_historical_merkle_proof,
        }
    }

    pub fn set_witness<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>>(
        &self,
        witness: &mut impl Witness<F>,
        end_cap_result: &UPSEndCapResultCompact<F>,
        guta_stats: &GUTAStats<F>,
        checkpoint_historical_merkle_proof: &MerkleProofCore<QHashOut<F>>,
        proof: &ProofWithPublicInputs<F, C, D>,
        verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) where
    <C as GenericConfig<D>>::Hasher:AlgebraicHasher<F>, {
        self.end_cap_result_gadget.set_witness(
            witness,
            end_cap_result,
        );
        self.guta_stats.set_witness(
            witness,
            guta_stats,
        );
        self.checkpoint_historical_merkle_proof.set_witness_proof_core(
            witness,
            checkpoint_historical_merkle_proof,
        );

        witness.set_proof_with_pis_target(&self.proof_target, &proof);
        witness.set_verifier_data_target(&self.verifier_data, &verifier_data);
    }
}

impl<const D: usize> ToGUTAHeader<D> for VerifyEndCapProofGadget<D> {
    fn get_guta_header<H: AlgebraicHasher<F>, F: RichField + Extendable<D>>(&self, builder: &mut CircuitBuilder<F, D>, default_guta_circuit_whitelist: HashOutTarget) -> GlobalUserTreeAggregatorHeaderGadget {
        GlobalUserTreeAggregatorHeaderGadget {
            guta_circuit_whitelist: default_guta_circuit_whitelist,
            checkpoint_tree_root: self.checkpoint_historical_merkle_proof.current_root,
            state_transition: SubTreeNodeStateTransitionGadget{
                old_node_value: self.end_cap_result_gadget.start_user_leaf_hash,
                new_node_value: self.end_cap_result_gadget.end_user_leaf_hash,
                node_index: self.end_cap_result_gadget.user_id,

                node_level: builder.constant_u8(GLOBAL_USER_TREE_HEIGHT),
                
            },
            stats: self.guta_stats.to_owned(),
        }
    }
}