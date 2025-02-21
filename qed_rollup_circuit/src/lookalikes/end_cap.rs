use plonky2::{hash::hash_types::HashOutTarget, plonk::{circuit_builder::CircuitBuilder, circuit_data::{CircuitConfig, CircuitData}, config::{AlgebraicHasher, GenericConfig}}};
use qed_common_circuit::{builder::{hash::core::CircuitBuilderHashCore, pad_circuit::{pad_circuit_degree, CircuitBuilderQEDCommonGates}}, proof_minifier::pm_core::get_circuit_fingerprint_generic};
use qed_core::data::qhashout::QHashOut;


pub struct EndCapLookalikeCircuit<C: GenericConfig<D>, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub input_hash: HashOutTarget,
    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}
impl<C: GenericConfig<D>, const D: usize> EndCapLookalikeCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let input_hash = builder.add_virtual_hash();
        let output_hash = builder.hash_two_to_one::<C::Hasher>(input_hash, input_hash);

        builder.register_public_inputs(&output_hash.elements);
        builder.add_qed_type_b_common_gates();
        pad_circuit_degree(&mut builder, 11);
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));

        Self {
            input_hash,
            circuit_data,
            fingerprint,
        }
    }
}