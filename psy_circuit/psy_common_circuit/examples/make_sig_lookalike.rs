use plonky2::{
    hash::hash_types::HashOutTarget,
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData},
        config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use psy_common_circuit::{
    builder::{
        hash::core::CircuitBuilderHashCore,
        pad_circuit::{pad_circuit_degree, CircuitBuilderPsyCommonGates},
    },
    circuits::{traits::qstandard::QStandardCircuit, zk_signature3::manager::SimplePsyZKSignatureManager},
    proof_minifier::pm_core::get_circuit_fingerprint_generic,
};

pub struct SimplerSigLookalikeCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub input_hash: HashOutTarget,
    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}
impl<C: GenericConfig<D> + 'static, const D: usize> SimplerSigLookalikeCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let input_hash = builder.add_virtual_hash();
        let output_hash = builder.hash_two_to_one::<C::Hasher>(input_hash, input_hash);

        builder.register_public_inputs(&output_hash.elements);
        //builder.add_psy_type_b_common_gates();
        //pad_circuit_degree(&mut builder, 11);
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            input_hash,
            circuit_data,
            fingerprint,
        }
    }

    pub fn prove_simple(&self, hash: QHashOut<C::F>) -> ProofWithPublicInputs<C::F, C, D> {
        let mut witness = PartialWitness::new();
        witness.set_hash_target(self.input_hash, hash.0).unwrap();

        self.circuit_data.prove(witness).unwrap()
    }
}
pub struct SimpleSigLookalikeCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub input_hash: HashOutTarget,
    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}
impl<C: GenericConfig<D> + 'static, const D: usize> SimpleSigLookalikeCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let input_hash = builder.add_virtual_hash();
        let output_hash = builder.hash_two_to_one::<C::Hasher>(input_hash, input_hash);

        builder.register_public_inputs(&output_hash.elements);
        builder.add_psy_type_b_common_gates();
        pad_circuit_degree(&mut builder, 11);
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            input_hash,
            circuit_data,
            fingerprint,
        }
    }

    pub fn prove_simple(&self, hash: QHashOut<C::F>) -> ProofWithPublicInputs<C::F, C, D> {
        let mut witness = PartialWitness::new();
        witness.set_hash_target(self.input_hash, hash.0).unwrap();

        self.circuit_data.prove(witness).unwrap()
    }
}
pub fn get_simple_sig_common_data<C: GenericConfig<D> + 'static, const D: usize>() -> CommonCircuitData<C::F, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<C::F, D>::new(config);
    let input_hash = builder.add_virtual_hash();
    let output_hash = builder.hash_two_to_one::<C::Hasher>(input_hash, input_hash);

    builder.register_public_inputs(&output_hash.elements);
    builder.add_psy_type_b_common_gates();
    pad_circuit_degree(&mut builder, 4);
    let circuit_data = builder.build::<C>();

    let mut common = circuit_data.common;
    common.fri_params.degree_bits = 12;
    common.fri_params.reduction_arity_bits = vec![4, 4];

    common
}

fn run_check_sig_lookalike() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("run_check_sig_lookalike");
    timer.lap("start");

    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;
    let simpler_circuit = SimplerSigLookalikeCircuit::<C, D>::new();
    timer.lap("built simpler circuit");
    let lookalike_circuit = SimpleSigLookalikeCircuit::<C, D>::new();
    timer.lap("built lookalike_circuit");
    let wallet = SimplePsyZKSignatureManager::<C, D>::new();
    timer.lap("built wallet");

    let get_lookalike_fast = get_simple_sig_common_data::<C, D>();
    timer.lap("got fast lookalike");

    //println!("\n\nlookalike_circuit.common (cap_height:
    // {}):\n{:?}\n\n",lookalike_circuit.circuit_data.verifier_only.
    // constants_sigmas_cap.height(), &lookalike_circuit.circuit_data.common);

    println!(
        "\n\nlookalike_circuit.common (cap_height: {}):\n{:?}\n\n",
        lookalike_circuit.circuit_data.verifier_only.constants_sigmas_cap.height(),
        &lookalike_circuit.circuit_data.common
    );
    println!(
        "\n\nlookalike_circuit2.common (cap_height: {}):\n{:?}\n\n",
        lookalike_circuit.circuit_data.verifier_only.constants_sigmas_cap.height(),
        &get_lookalike_fast
    );
    println!(
        "\n\nsignature_circuit.common (cap_height: {}):\n{:?}\n\n",
        wallet.circuit.get_verifier_config_ref().constants_sigmas_cap.height(),
        &wallet.circuit.get_common_circuit_data_ref()
    );

    println!("\nspeed info\n");
    timer.lap("start speed info");

    simpler_circuit.prove_simple(QHashOut::rand());
    timer.lap("proved simpler");

    lookalike_circuit.prove_simple(QHashOut::rand());
    timer.lap("proved lookalike");

    Ok(())
}

fn main() {
    run_check_sig_lookalike().unwrap();
}
