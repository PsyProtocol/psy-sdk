use hashbrown::HashMap;
use plonky2::{hash::hash_types::RichField, plonk::{circuit_data::VerifierOnlyCircuitData, config::{AlgebraicHasher, GenericConfig}}};
use qed_core::{data::{alt::AltVerifierOnlyCircuitData, qhashout::QHashOut}, job::id::ProvingJobCircuitType};
use serde::{Deserialize, Serialize};

use crate::hash::merkle::core::MerkleProofCore;

use super::circuit_library::{CircuitInfoLibrary, CircuitInfoLibraryBuilder};


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct BasicCircuitInfo<F: RichField> {
    pub verifier_data: AltVerifierOnlyCircuitData<F>,
    pub fingerprint: QHashOut<F>,
}


#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CircuitTypeInclusionMappingKey {
    pub parent: ProvingJobCircuitType,
    pub child: ProvingJobCircuitType,
}


#[derive(Clone, Debug, PartialEq)]
pub struct SimpleCircuitLibrary<F: RichField> {
    pub info_map: HashMap<ProvingJobCircuitType, BasicCircuitInfo<F>>,
    pub inclusion_proofs: Vec<MerkleProofCore<QHashOut<F>>>,
    pub inclusion_map: HashMap<CircuitTypeInclusionMappingKey, usize>,


}
impl<F: RichField> SimpleCircuitLibrary<F>{
    fn internal_get_basic_info(&self, circuit_type: ProvingJobCircuitType) -> anyhow::Result<&BasicCircuitInfo<F>> {
        match self.info_map.get(&circuit_type) {
            Some(x) => Ok(x),
            None => anyhow::bail!("circuit type '{:?}' not registered", circuit_type),
        }
    }
    fn internal_register_circuit(&mut self, circuit_type: ProvingJobCircuitType, fingerprint: QHashOut<F>, verifier_data: AltVerifierOnlyCircuitData<F>) {
        self.info_map.insert(circuit_type, BasicCircuitInfo{
            verifier_data,
            fingerprint,
        });
    }
    fn internal_add_inclusion_proof(&mut self, parent_types: &[ProvingJobCircuitType], child_type: ProvingJobCircuitType, proof: MerkleProofCore<QHashOut<F>>) {
        let ind = self.inclusion_proofs.len();
        self.inclusion_proofs.push(proof);
        for t in parent_types {
            self.inclusion_map.insert(CircuitTypeInclusionMappingKey{
                parent: *t,
                child: child_type,
            }, ind);
        }
    }
    fn _internal_register_combo(&mut self, circuit_type: ProvingJobCircuitType, verifier_data: AltVerifierOnlyCircuitData<F>, parent_types: &[ProvingJobCircuitType], proof: MerkleProofCore<QHashOut<F>>) {
        self.info_map.insert(circuit_type, BasicCircuitInfo{
            verifier_data,
            fingerprint: proof.value,
        });
        let ind = self.inclusion_proofs.len();
        self.inclusion_proofs.push(proof);
        for t in parent_types {
            self.inclusion_map.insert(CircuitTypeInclusionMappingKey{
                parent: *t,
                child: circuit_type,
            }, ind);
        }
    }
    fn internal_get_inclusion_proof(&self, parent_type: ProvingJobCircuitType, child_type: ProvingJobCircuitType) -> anyhow::Result<&MerkleProofCore<QHashOut<F>>> {
        match self.inclusion_map.get(&CircuitTypeInclusionMappingKey{
            parent: parent_type,
            child: child_type,
        }){
            Some(v) => Ok(&self.inclusion_proofs[*v]),
            None => anyhow::bail!("could not find inclusion proof for parent = {:?}, child = {:?}",parent_type, child_type),
        }
    }
}

impl<F: RichField> CircuitInfoLibraryBuilder<F> for SimpleCircuitLibrary<F> {
    fn register_circuit(&mut self, circuit_type: ProvingJobCircuitType, fingerprint: QHashOut<F>, verifier_data: AltVerifierOnlyCircuitData<F>) {
        self.internal_register_circuit(circuit_type, fingerprint, verifier_data);
    }

    fn add_inclusion_proof(&mut self, parent_types: &[ProvingJobCircuitType], child_type: ProvingJobCircuitType, proof: MerkleProofCore<QHashOut<F>>) {
        self.internal_add_inclusion_proof(parent_types, child_type, proof);
    }
}

impl<C: GenericConfig<D>, const D: usize> CircuitInfoLibrary<C,D> for SimpleCircuitLibrary<C::F>  where C::Hasher: AlgebraicHasher<C::F> {
    
    fn get_verifier_data(&self, circuit_type: ProvingJobCircuitType) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        Ok(self.internal_get_basic_info(circuit_type)?.verifier_data.to_verifier_data::<C,D>())
    }
    fn get_fingerprint(&self, circuit_type: ProvingJobCircuitType) -> anyhow::Result<QHashOut<C::F>>{
        Ok(self.internal_get_basic_info(circuit_type)?.fingerprint)
    }
    fn get_group_inclusion_proof(&self, parent_circuit: ProvingJobCircuitType, proof_circuit_type: ProvingJobCircuitType) -> anyhow::Result<MerkleProofCore<QHashOut<C::F>>> {
        Ok(self.internal_get_inclusion_proof(parent_circuit, proof_circuit_type)?.to_owned())
    }  
}