use std::time::Instant;

use plonky2::{
    hash::hash_types::HashOut,
    plonk::{
        circuit_data::VerifierOnlyCircuitData,
        config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_core::{
    config::network_constants::QED_NETWORK_MAGIC_REGTEST,
    data::{alt::AltVerifierOnlyCircuitData, qhashout::QHashOut},
    utils::debug_timer::DebugTimer,
};
use psy_crypto::{
    common::witnesses::qrecursion::header::QRecursionAggStandardHeader,
    hash::{merkle::core::MerkleProofCore, traits::hasher::MerkleZeroHasher},
};
use psy_data::ups::ups_end_cap::UPSEndCapFromProofTreeGadgetInput;
use psy_prover::ups::circuit_manager::core::QEDUPSStepCircuitManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct DebugEndCapPerfInputCore<C: GenericConfig<D>, const D: usize> {
    pub end_cap_from_proof_tree_input: UPSEndCapFromProofTreeGadgetInput<C::F>,
    pub agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<C::F>>,
    pub agg_proof_header: QRecursionAggStandardHeader<C::F>,
    pub agg_root_proof: ProofWithPublicInputs<C::F, C, D>,
}
pub struct DebugEndCapPerfInputReady<C: GenericConfig<D>, const D: usize> {
    pub core: DebugEndCapPerfInputCore<C, D>,
    pub agg_root_verifier_data: VerifierOnlyCircuitData<C, D>,
}
impl<C: GenericConfig<D>, const D: usize> DebugEndCapPerfInputReady<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub fn into_ser(self) -> DebugEndCapPerfInputSer<C, D> {
        DebugEndCapPerfInputSer {
            core: self.core,
            agg_root_verifier_data: AltVerifierOnlyCircuitData::new_from_verifier_data(&self.agg_root_verifier_data),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct DebugEndCapPerfInputSer<C: GenericConfig<D>, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub core: DebugEndCapPerfInputCore<C, D>,
    pub agg_root_verifier_data: AltVerifierOnlyCircuitData<C::F>,
}

impl<C: GenericConfig<D>, const D: usize> DebugEndCapPerfInputSer<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub fn into_ready(self) -> DebugEndCapPerfInputReady<C, D> {
        DebugEndCapPerfInputReady {
            core: self.core,
            agg_root_verifier_data: self.agg_root_verifier_data.to_verifier_data::<C, D>(),
        }
    }
}
impl<C: GenericConfig<D>, const D: usize> From<DebugEndCapPerfInputSer<C, D>> for DebugEndCapPerfInputReady<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    fn from(value: DebugEndCapPerfInputSer<C, D>) -> Self {
        Self {
            core: value.core,
            agg_root_verifier_data: value.agg_root_verifier_data.to_verifier_data::<C, D>(),
        }
    }
}

impl<C: GenericConfig<D>, const D: usize> From<DebugEndCapPerfInputReady<C, D>> for DebugEndCapPerfInputSer<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    fn from(value: DebugEndCapPerfInputReady<C, D>) -> Self {
        Self {
            core: value.core,
            agg_root_verifier_data: AltVerifierOnlyCircuitData::new_from_verifier_data(&value.agg_root_verifier_data),
        }
    }
}

struct EndCapTestBench<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    step_circuit_mgr: QEDUPSStepCircuitManager<C, D>,
    pub end_cap_proving_times: Vec<u64>,
    total_proving_time: u64,
    run_iterations: u64,
}

impl<C: GenericConfig<D> + 'static, const D: usize> EndCapTestBench<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub fn new() -> Self {
        Self {
            step_circuit_mgr: QEDUPSStepCircuitManager::new_with_config(QED_NETWORK_MAGIC_REGTEST),
            end_cap_proving_times: Vec::new(),
            run_iterations: 0,
            total_proving_time: 0,
        }
    }
    pub fn get_average_proving_time_ms(&self) -> u64 {
        ((self.total_proving_time as f64) / (self.run_iterations as f64)) as u64
    }
    pub fn run_bench_end_cap(&mut self, input_data: &DebugEndCapPerfInputReady<C, D>) -> anyhow::Result<()> {
        let start_time = Instant::now();

        self.step_circuit_mgr.ups_end_cap.prove_base(
            &input_data.core.end_cap_from_proof_tree_input,
            &input_data.core.agg_whitelist_merkle_proof,
            &input_data.core.agg_proof_header,
            &input_data.core.agg_root_proof,
            &input_data.agg_root_verifier_data,
        )?;
        let duration = start_time.elapsed();
        let t = duration.as_millis() as u64;
        self.total_proving_time += t;
        self.run_iterations += 1;

        self.end_cap_proving_times.push(t);

        Ok(())
    }
}

fn main1() -> anyhow::Result<()> {
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    let mut timer = DebugTimer::new("bench end_cap");

    let end_caps: Vec<DebugEndCapPerfInputSer<C, D>> = vec![]; //serde_json::from_str::<Vec<DebugEndCapPerfInputSer<C,
                                                               // D>>>(include_str!("./debug_data/end_cap_dbg_2.json"))?;

    let end_caps = end_caps.into_iter().map(|x| x.into_ready()).collect::<Vec<_>>();
    timer.lap("deserialized_end_caps");

    let mut test_bench = EndCapTestBench::<C, D>::new();
    timer.lap("built circuits for test bench");

    const TEST_ITERATIONS: usize = 10;

    for _ in 0..TEST_ITERATIONS {
        for ec in end_caps.iter() {
            test_bench.run_bench_end_cap(ec)?;
        }
        println!("average time: {}", test_bench.get_average_proving_time_ms());
    }
    println!("all_times: {:?}", test_bench.end_cap_proving_times);
    println!("average time: {}", test_bench.get_average_proving_time_ms());

    Ok(())
}

fn main() {
    main1().unwrap();
}
