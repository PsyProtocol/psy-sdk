use plonky2::{
    field::types::Field, gates::gate::GateRef, hash::hash_types::{HashOut, HashOutTarget}, iop::witness::{PartialWitness, WitnessWrite}, plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    }
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::{common::witnesses::zk_signature::QEDZKSignatureCircuitInput, hash::traits::hasher::MerkleZeroHasher, signature::zk::wallet::PRIVATE_KEY_CONSTANTS};

use crate::{
    builder::hash::core::CircuitBuilderHashCore, proof_minifier::pm_chain::QEDProofMinifierChain, u32::gates::comparison::ComparisonGate,
};

use super::super::traits::qstandard::{provable::QStandardCircuitProvable, QStandardCircuit};
#[derive(Debug)]
pub struct QEDBasicZKSignatureCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub private_key: HashOutTarget,
    pub sig_hash: HashOutTarget,
    // end circuit targets
    pub minifier_chain: QEDProofMinifierChain<D, C::F, C>,
    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}
impl<C: GenericConfig<D>, const D: usize> Clone for QEDBasicZKSignatureCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    fn clone(&self) -> Self {
        Self::new()
    }
}
impl<C: GenericConfig<D>, const D: usize> QEDBasicZKSignatureCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_zk_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let private_key = builder.add_virtual_hash();
        let private_key_constants = PRIVATE_KEY_CONSTANTS
            .iter()
            .map(|c| builder.constant(C::F::from_canonical_u64(*c)))
            .collect::<Vec<_>>();
        let public_key_param_target = builder.hash_n_to_hash_no_pad::<C::Hasher>(vec![
            private_key_constants[0],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[19],
            private_key.elements[1],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[3],
            private_key_constants[4],
            private_key_constants[5],
            private_key_constants[6],
            private_key.elements[0],
            private_key_constants[7],
            private_key.elements[2],
            private_key_constants[8],
            private_key_constants[9],
            private_key_constants[10],
            private_key_constants[11],
            private_key_constants[12],
            private_key.elements[3],
            private_key_constants[13],
            private_key_constants[14],
            private_key_constants[15],
            private_key_constants[16],
            private_key_constants[17],
            private_key_constants[18],
        ]);

        let sig_hash = builder.add_virtual_hash();
        let public_inputs_hash = builder.hash_two_to_one::<C::Hasher>(
            sig_hash,
            public_key_param_target, 
        );
        builder.register_public_inputs(&public_inputs_hash.elements);
        let circuit_data = builder.build::<C>();

        // start add some gates to make it easier to integrate with others

        let added_gates_for_minifier = [
            GateRef::new(ComparisonGate::new(32, 16)),
        ];

        let minifier_chain = QEDProofMinifierChain::<D, C::F, C>::new_add_gates(
            &circuit_data.verifier_only,
            &circuit_data.common,
            2,
            Some(&added_gates_for_minifier),
        );
        let fingerprint = QHashOut(minifier_chain.get_fingerprint());
        Self {
            private_key,
            sig_hash,
            circuit_data,
            minifier_chain: minifier_chain,
            fingerprint,
        }
    }
    pub fn prove_base(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::new();
        pw.set_hash_target(self.private_key, private_key.0)?;
        pw.set_hash_target(self.sig_hash, sig_hash.0)?;
        let inner_proof = self.circuit_data.prove(pw)?;
        self.minifier_chain.prove(&inner_proof)
    }
}
impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D> for QEDBasicZKSignatureCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        self.fingerprint
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        self.minifier_chain.get_verifier_data()
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        self.minifier_chain.get_common_data()
    }
}
impl<C: GenericConfig<D>, const D: usize>
    QStandardCircuitProvable<QEDZKSignatureCircuitInput<C::F>, C, D> for QEDBasicZKSignatureCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn prove_standard(
        &self,
        input: &QEDZKSignatureCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.prove_base(input.private_key, input.sig_hash)
    }
}
