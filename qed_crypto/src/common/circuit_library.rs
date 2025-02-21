use plonky2::{hash::hash_types::RichField, plonk::{circuit_data::VerifierOnlyCircuitData, config::GenericConfig}};
use qed_core::{data::{alt::AltVerifierOnlyCircuitData, qhashout::QHashOut}, job::id::ProvingJobCircuitType};

use crate::hash::merkle::core::MerkleProofCore;


pub trait CircuitInfoLibraryBuilder<F: RichField> {
    fn register_circuit(&mut self, circuit_type: ProvingJobCircuitType, fingerprint: QHashOut<F>, verifier_data: AltVerifierOnlyCircuitData<F>);
    fn add_inclusion_proof(&mut self, parent_types: &[ProvingJobCircuitType], child_type: ProvingJobCircuitType, proof: MerkleProofCore<QHashOut<F>>);
}

pub trait CircuitInfoLibrary<C: GenericConfig<D>, const D: usize> {
    fn get_verifier_data(&self, circuit_type: ProvingJobCircuitType) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;
    fn get_fingerprint(&self, circuit_type: ProvingJobCircuitType) -> anyhow::Result<QHashOut<C::F>>;
    fn get_group_inclusion_proof(&self, parent_circuit: ProvingJobCircuitType, proof_circuit_type: ProvingJobCircuitType) -> anyhow::Result<MerkleProofCore<QHashOut<C::F>>>;    
}