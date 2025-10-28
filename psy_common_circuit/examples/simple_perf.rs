use std::time::Instant;

use plonky2::{
    hash::hash_types::HashOutTarget, iop::
        witness::{PartialWitness, WitnessWrite}
    , plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    }
};
use psy_common_circuit::{
    builder::hash::core::CircuitBuilderHashCore,
    circuits::traits::qstandard::QStandardCircuit,
    proof_minifier::{
        pm_chain_dynamic::QEDProofMinifierDynamicChain, pm_core::get_circuit_fingerprint_generic,
    },
};
use psy_core::data::qhashout::QHashOut;

#[derive(Debug)]
pub struct SimplePerfCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub input_hash: HashOutTarget,

    // end circuit targets
    pub base_circuit_data: CircuitData<C::F, C, D>,
    pub base_fingerprint: QHashOut<C::F>,
    pub minifier_chain: Option<QEDProofMinifierDynamicChain<D, C::F, C>>,
    pub enable_minifier: bool,
    // end circuit data
}

impl<C: GenericConfig<D> + 'static, const D: usize> SimplePerfCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> {
    pub fn new_with_minifier() -> Self {
        Self::new_with_config(512, true)
    }
    pub fn new_without_minifier() -> Self {
        Self::new_with_config(512, true)
    }
    pub fn new_with_config(
        hash_count: usize,
        has_minifier: bool,
    ) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let input_hash = builder.add_virtual_hash();
        let mut current_hash = input_hash;
        for i in 0..hash_count {
            let t_hash = builder.constant_qhash(QHashOut::from_values(i as u64, i as u64, i as u64, i as u64));
            current_hash = builder.hash_two_to_one::<C::Hasher>(current_hash, t_hash);
        }

        builder.register_public_inputs(&current_hash.elements);
        let base_circuit_data = builder.build::<C>();

        let base_fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &base_circuit_data.verifier_only,
        ));

        let minifier_chain = if has_minifier {
            Some(QEDProofMinifierDynamicChain::<D, C::F, C>::new_with_dynamic_constant_verifier(
                &base_circuit_data.verifier_only,
                &base_circuit_data.common,
                &[true, false],
            ))
        }else{
            None
        };

        Self {
            input_hash,
            base_circuit_data,
            base_fingerprint,
            minifier_chain,
            enable_minifier: has_minifier,
        }
    }
    
    pub fn is_minifier_enabled(&self) -> bool {
        self.enable_minifier && self.minifier_chain.is_some()
    }

    fn prove_base_inner(
        &self,
        input_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw_base = PartialWitness::<C::F>::new();
        pw_base.set_hash_target(self.input_hash, input_hash.0)?;
        self.base_circuit_data.prove(pw_base)

    }
    pub fn prove_base(
        &self,
        input_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        if self.is_minifier_enabled() {
            let base_proof = self.prove_base_inner(input_hash)?;
            self.minifier_chain.as_ref().unwrap().prove(&base_proof)
        }else{
            self.prove_base_inner(input_hash)
        }
    }
    
}

impl<C: GenericConfig<D> + 'static, const D: usize> QStandardCircuit<C, D>
    for SimplePerfCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        if self.is_minifier_enabled() {
            QHashOut(self.minifier_chain.as_ref().unwrap().get_fingerprint())
        }else{
            self.base_fingerprint
        }
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        if self.is_minifier_enabled() {
            self.minifier_chain.as_ref().unwrap().get_verifier_data()
        }else{
            &self.base_circuit_data.verifier_only
        }
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        if self.is_minifier_enabled() {
            self.minifier_chain.as_ref().unwrap().get_common_data()
        }else{
            &self.base_circuit_data.common
        }
    }
}


struct TestBench<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:
        AlgebraicHasher<C::F> 
{
    circuit: SimplePerfCircuit<C, D>,
    pub proving_times: Vec<u64>,
    total_proving_time: u64,
    run_iterations: u64,
}

impl<C: GenericConfig<D> + 'static, const D: usize> TestBench<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F>
{
    pub fn new(
        hash_count: usize,
        has_minifier: bool,
    ) -> Self {
        Self {
            circuit: SimplePerfCircuit::new_with_config(hash_count, has_minifier),
            proving_times: Vec::new(),
            run_iterations: 0,
            total_proving_time: 0,
        }
    }
    pub fn get_average_proving_time_ms(&self) -> u64 {
        ((self.total_proving_time as f64)/(self.run_iterations as f64)) as u64
    }
    pub fn run_bench_end_cap(
        &mut self,
        input_data: QHashOut<C::F>,
    ) -> anyhow::Result<()> {
        let start_time = Instant::now();

        self.circuit.prove_base(input_data)?;
        let duration = start_time.elapsed();
        let t = duration.as_millis() as u64;
        self.total_proving_time += t;
        self.run_iterations += 1;

        self.proving_times.push(t);

        Ok(())
    }
}

fn run_simple_perf() -> anyhow::Result<()> {
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    let mut spc = TestBench::<C,D>::new(2048, true);
    println!("fingerprint: {:?}",spc.circuit.get_fingerprint());

    for _ in 0..64 {
        spc.run_bench_end_cap(QHashOut::rand())?;
        println!("avg time: {}", spc.get_average_proving_time_ms());
    }

    println!("proving_times: {:?}", spc.proving_times);


    Ok(())
}
fn main() {
    run_simple_perf().unwrap();

}