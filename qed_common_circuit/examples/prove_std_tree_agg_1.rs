use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{PrimeField64, Sample},
    },
    hash::poseidon::PoseidonHash,
    iop::witness::PartialWitness,
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData},
        config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_common_circuit::{
    builder::hash::core::CircuitBuilderHashCore,
    hash::merkle::gadgets::merkle_proof::MerkleProofGadget,
    proof_minifier::pm_core::get_circuit_fingerprint_generic,
    treeprover::qrecursion::standard::manager::simple::simple::SimpleQTreeRecursionManager,
};
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_crypto::{
    common::witnesses::qrecursion::proof_data::InputLeafProof,
    hash::merkle::{core::MerkleProofCore, utils::simple_merkle_tree::SimpleMerkleTree},
};

pub struct SimpleLeafCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub mp_gadget: MerkleProofGadget,
    pub base_circuit_data: CircuitData<C::F, C, D>,
    pub base_fingerprint: QHashOut<C::F>,
}
impl<C: GenericConfig<D> + 'static, const D: usize> SimpleLeafCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub fn new(merkle_tree_height: usize) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let mp_gadget = MerkleProofGadget::add_virtual_to::<C::Hasher, C::F, D>(
            &mut builder,
            merkle_tree_height,
        );
        let combined_state_hash =
            builder.hash_two_to_one::<C::Hasher>(mp_gadget.root, mp_gadget.value);

        builder.register_public_inputs(&combined_state_hash.elements);

        let base_circuit_data = builder.build::<C>();

        let base_fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &base_circuit_data.verifier_only,
        ));

        Self {
            mp_gadget,
            base_circuit_data,
            base_fingerprint,
        }
    }

    pub fn prove_base(
        &self,
        merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
    ) -> ProofWithPublicInputs<C::F, C, D> {
        let mut witness = PartialWitness::new();
        self.mp_gadget
            .set_witness_core_proof_q(&mut witness, merkle_proof);

        self.base_circuit_data.prove(witness).unwrap()
    }
}

fn run_prove_agg_example_1() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("run_prove_agg_example_1");
    timer.lap("start");

    type F = GoldilocksField;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    let mtree_height: usize = 60;
    let mtree_index_mask = (1u64 << (mtree_height as u64)) - 1u64;
    let proof_tree_height: usize = 16;
    let ex_leaf_circuit = SimpleLeafCircuit::<C, D>::new(mtree_height);
    timer.lap("built SimpleLeafCircuit");

    let mut merkle_tree = SimpleMerkleTree::<PoseidonHash, QHashOut<F>>::new(mtree_height as u8);
    let mut inds: Vec<u64> = Vec::new();
    for i in 0..256 {
        let rand_value = QHashOut::<F>::rand();
        let rand_index = (((F::rand().to_noncanonical_u64()) << 8u64) + i) & mtree_index_mask;
        inds.push(rand_index);
        merkle_tree.set_leaf(rand_index, rand_value);
    }
    timer.lap("set leaves in tree");

    let mut recursion_mgr = SimpleQTreeRecursionManager::new(
        proof_tree_height,
        1337,
        ex_leaf_circuit
            .base_circuit_data
            .verifier_only
            .constants_sigmas_cap
            .height(),
        &ex_leaf_circuit.base_circuit_data.common,
    );
    timer.lap("created SimpleQTreeRecursionManager");
    let start_proof_tree_root = recursion_mgr.get_proof_tree_root();

    //recursion_mgr.circuit_set.print_common_data();
    let input_leaf_items = (0..6)
        .map(|i| {
            let leaf_proof = ex_leaf_circuit.prove_base(&merkle_tree.get_leaf(inds[i]));
            let input_item = InputLeafProof {
                fingerprint: ex_leaf_circuit.base_fingerprint,
                proof: leaf_proof,
                verifier_data: ex_leaf_circuit.base_circuit_data.verifier_only.clone(),
            };
            input_item
        })
        .collect::<Vec<_>>();
    timer.lap("proved input leaf items");
    let _leaf_mgr_inds = recursion_mgr.add_leaf_proofs(input_leaf_items);
    timer.lap("added leaf proofs");
    /*
    recursion_mgr.prove_one_step_simple_serial();
    recursion_mgr.prove_one_step_simple_serial();
    recursion_mgr.prove_one_step_simple_serial();
    recursion_mgr.prove_one_step_simple_serial();*/
    recursion_mgr.finalize_tree()?;
    timer.lap("finalized tree");
    println!(
        "recursion_mgr.leaf_proofs.len() = {}",
        recursion_mgr.leaf_proofs.len()
    );
    println!(
        "recursion_mgr.agg_proofs.len() = {}",
        recursion_mgr.agg_proofs.len()
    );

    let final_proof = &recursion_mgr.agg_proofs[0];
    println!("final_proof.agg_header: {:?}", &final_proof.agg_header);
    let final_proof_tree_root = recursion_mgr.get_proof_tree_root();

    assert_eq!(
        final_proof.agg_header.state_transition_start, start_proof_tree_root,
        "agg_header.state_transition_start should equal start_proof_tree_root"
    );
    assert_eq!(
        final_proof.agg_header.state_transition_end, final_proof_tree_root,
        "agg_header.state_transition_end should equal final_proof_tree_root"
    );

    Ok(())
}

fn main() {
    run_prove_agg_example_1().unwrap();
}
