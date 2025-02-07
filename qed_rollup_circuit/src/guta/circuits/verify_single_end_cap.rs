use plonky2::{
    hash::hash_types::{HashOut, HashOutTarget}, iop::
        witness::{PartialWitness, WitnessWrite}
    , plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    }
};
use qed_common_circuit::{
    builder::hash::core::CircuitBuilderHashCore, circuits::traits::qstandard::QStandardCircuit, proof_minifier::
        pm_core::get_circuit_fingerprint_generic
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
use qed_data::guta::proof_input::VerifySingleEndCapInput;

use crate::guta::gadgets::{helpers::ToGUTAHeader, verify_end_cap::VerifyEndCapProofGadget};

#[derive(Debug)]
pub struct GUTAVerifySingleEndCapCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub guta_circuit_whitelist_root_hash: HashOutTarget,
    pub a_end_cap_gadget: VerifyEndCapProofGadget<D>,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> GUTAVerifySingleEndCapCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> {
        pub fn new(
            end_cap_proof_common_data: &CommonCircuitData<C::F, D>,
            end_cap_proof_verifier_data_cap_height: usize,
            known_end_cap_fingerprint: QHashOut<C::F>,
        ) -> Self {

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let known_end_cap_fingerprint_hash = builder.constant_qhash(known_end_cap_fingerprint);

        let guta_circuit_whitelist_root_hash = builder.add_virtual_hash();

        let a_end_cap_gadget = VerifyEndCapProofGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint_hash,
        );

        let a_end_cap_guta_header = a_end_cap_gadget.get_guta_header::<C::Hasher, C::F>(
            &mut builder,
            guta_circuit_whitelist_root_hash,
        );

        let public_inputs_hash = a_end_cap_guta_header.to_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&public_inputs_hash.elements);

        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));

        Self {
            guta_circuit_whitelist_root_hash,
            a_end_cap_gadget,
            circuit_data,
            fingerprint,
        }
    }
    
    pub fn prove_base(
        &self,
        input: &VerifySingleEndCapInput<C::F>,
        child_a_proof: &ProofWithPublicInputs<C::F, C, D>,
        end_cap_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();
        pw.set_hash_target(self.guta_circuit_whitelist_root_hash, input.guta_circuit_whitelist.0);

        self.a_end_cap_gadget.set_witness(
            &mut pw,
            &input.get_end_result_a(),
            &input.a_end_cap.guta_stats,
            &input.a_end_cap.checkpoint_historical_merkle_proof,
            child_a_proof,
            end_cap_verifier_data
        );

        self.circuit_data.prove(pw)
    }
}


impl<C: GenericConfig<D> + 'static, const D: usize> QStandardCircuit<C, D>
    for GUTAVerifySingleEndCapCircuit<C, D>
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

