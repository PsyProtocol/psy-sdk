use std::collections::HashMap;

use plonky2::
    plonk::{
        circuit_data::{CommonCircuitData, VerifierOnlyCircuitData},
        config::GenericConfig,
        proof::ProofWithPublicInputs,
        verifier_v2::verify_standard_proof,
    }
;
use qed_core::job::id::ProvingJobCircuitType;

#[derive(Debug, Clone)]
pub struct VerifierDataPair<C: GenericConfig<D>, const D: usize> {
    pub circuit_type: ProvingJobCircuitType,
    pub verifier_data: VerifierOnlyCircuitData<C, D>,
}
#[derive(Debug, Clone)]
pub struct CommonConfigVerifier<C: GenericConfig<D>, const D: usize> {
    pub common_circuit_data: CommonCircuitData<C::F, D>,
    pub circuits: Vec<VerifierDataPair<C, D>>,
}

#[derive(Debug, Clone)]
pub struct GenericCircuitVerifier<C: GenericConfig<D>, const D: usize> {
    pub configs: Vec<CommonConfigVerifier<C, D>>,

    pub verifier_data_map: HashMap<ProvingJobCircuitType, (usize, usize)>,
}

impl<C: GenericConfig<D>, const D: usize> GenericCircuitVerifier<C, D> {
    pub fn new(configs: Vec<CommonConfigVerifier<C, D>>) -> Self {
        let mut verifier_data_map = HashMap::new();

        for (config_index, c) in configs.iter().enumerate() {
            for (verifier_data_index, v) in c.circuits.iter().enumerate() {
                verifier_data_map.insert(v.circuit_type, (config_index, verifier_data_index));
            }
        }

        Self {
            configs,
            verifier_data_map,
        }
    }

    pub fn verify_proof_of_type(
        &self,
        circuit_type: ProvingJobCircuitType,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        match self.verifier_data_map.get(&circuit_type) {
            Some(x) => {
                let (config_index, verifier_data_index) = *x;

                verify_standard_proof(
                    proof,
                    &self.configs[config_index].circuits[verifier_data_index].verifier_data,
                    &self.configs[config_index].common_circuit_data,
                )?;
            }
            None => anyhow::bail!("missing verifier data for type {:?}", circuit_type),
        }
        Ok(())
    }
}
