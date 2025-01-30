use plonky2::{field::extension::Extendable, hash::hash_types::RichField, plonk::circuit_data::CommonCircuitData};
use qed_core::data::qhashout::QHashOut;



pub trait QCircuitConfigResolver<ID, F: RichField + Extendable<D>, const D: usize> {
    fn contains_circuit_id(&self, circuit_id: &ID) -> bool;
    fn get_common_circuit_data_ref(&self, circuit_id: &ID) -> anyhow::Result<&CommonCircuitData<F, D>>;
    fn get_fingerprint(&self, circuit_id: &ID) -> QHashOut<F>;
}