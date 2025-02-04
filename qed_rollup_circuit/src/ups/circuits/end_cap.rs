use plonky2::{
    hash::hash_types::HashOut, iop::
        witness::PartialWitness
    , plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    }
};
use qed_common_circuit::{
    builder::hash::core::CircuitBuilderHashCore, circuits::traits::qstandard::QStandardCircuit, proof_minifier::
        pm_core::get_circuit_fingerprint_generic, treeprover::qrecursion::standard::gadgets::verify_agg_proof::VerifyAggProofGadget
};
use qed_core::{config::network_constants::{UPS_CIRCUIT_WHITELIST_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT}, data::qhashout::QHashOut};
use qed_crypto::{common::witnesses::qrecursion::header::QRecursionAggStandardHeader, hash::{merkle::core::MerkleProofCore, traits::hasher::MerkleZeroHasher}};
use qed_data::ups::ups_end_cap::UPSEndCapFromProofTreeGadgetInput;

use crate::ups::gadgets::ups_end_cap_tree::UPSEndCapFromProofTreeGadget;

#[derive(Debug)]
pub struct UPSStandardEndCapCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub end_cap_from_proof_tree_gadget: UPSEndCapFromProofTreeGadget,
    pub verify_proof_tree_root_gadget: VerifyAggProofGadget<D>,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> UPSStandardEndCapCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> {
        pub fn new(
            proof_tree_agg_common_data: &CommonCircuitData<C::F, D>,
            proof_tree_agg_verifier_data_cap_height: usize,
            network_magic: u64,
            known_ups_circuit_whitelist_root: QHashOut<C::F>,
            known_proof_tree_circuit_whitelist_root: QHashOut<C::F>,
        ) -> Self {
            Self::new_with_config(
                proof_tree_agg_common_data,
                proof_tree_agg_verifier_data_cap_height,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
                UPS_CIRCUIT_WHITELIST_TREE_HEIGHT as usize,
                network_magic,
                known_ups_circuit_whitelist_root,
                known_proof_tree_circuit_whitelist_root,
            )
        }
    pub fn new_with_config(
        proof_tree_agg_common_data: &CommonCircuitData<C::F, D>,
        proof_tree_agg_verifier_data_cap_height: usize,
        //coset_gate: &GateRef<C::F, D>,
        ups_session_proof_tree_height: usize,
        ups_circuit_whitelist_tree_height: usize,
        network_magic: u64,
        known_ups_circuit_whitelist_root: QHashOut<C::F>,
        known_proof_tree_circuit_whitelist_root: QHashOut<C::F>,
    ) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let verify_proof_tree_root_gadget = VerifyAggProofGadget::add_virtual_to::<C, C::F>(
            &mut builder,
            proof_tree_agg_common_data,
            proof_tree_agg_verifier_data_cap_height,
        );

        let known_proof_tree_circuit_whitelist_root_target = builder.constant_qhash(
            known_proof_tree_circuit_whitelist_root
        );


        // ensure the proof tree is using the correct, unmodified aggregation circuits
        builder.connect_hashes(
            known_proof_tree_circuit_whitelist_root_target,
            verify_proof_tree_root_gadget.agg_whitelist_merkle_proof.root,
        );


        let end_cap_from_proof_tree_gadget = UPSEndCapFromProofTreeGadget::add_virtual_to::<C::Hasher,C::F,D>(
            &mut builder,
            ups_session_proof_tree_height,
            ups_circuit_whitelist_tree_height,
            network_magic,
        );


        let known_ups_circuit_whitelist_root_target = builder.constant_qhash(
            known_ups_circuit_whitelist_root
        );

        // ensure the ups steps are using the correct ups circuit whitelist root
        builder.connect_hashes(
            known_ups_circuit_whitelist_root_target,
            end_cap_from_proof_tree_gadget.verify_previous_ups_step_gadget.ups_step_circuit_whitelist_root,
        );


        // ensure the proof tree proof's root matches the ups gadget's root
        builder.connect_hashes(
            verify_proof_tree_root_gadget.agg_proof_header_gadget.state_transition_end,
            end_cap_from_proof_tree_gadget.current_proof_tree_root,
        );




        let public_inputs_hash = end_cap_from_proof_tree_gadget.end_cap_core_gadget.end_cap_result_gadget.to_hash::<C::Hasher, C::F, D>(&mut builder);
        

        builder.register_public_inputs(&public_inputs_hash.elements);

        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));

        Self {
            end_cap_from_proof_tree_gadget,
            verify_proof_tree_root_gadget,
            circuit_data,
            fingerprint,
        }
    }
    
    pub fn prove_base(
        &self,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        agg_proof_header: &QRecursionAggStandardHeader<C::F>,
        agg_root_proof: &ProofWithPublicInputs<C::F, C, D>,
        agg_root_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        self.end_cap_from_proof_tree_gadget.set_witness(
            &mut pw,
            end_cap_from_proof_tree_input
        );
        self.verify_proof_tree_root_gadget.set_witness(
            &mut pw,
            agg_whitelist_merkle_proof,
            agg_proof_header,
            agg_root_proof,
            agg_root_verifier_data,
        );


        self.circuit_data.prove(pw)
    }
}


impl<C: GenericConfig<D> + 'static, const D: usize> QStandardCircuit<C, D>
    for UPSStandardEndCapCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        self.fingerprint
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        &self.circuit_data.verifier_only
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        &self.circuit_data.common
    }
}

