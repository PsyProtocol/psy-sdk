use crate::builder::{comparison::CircuitBuilderComparison, hash::core::CircuitBuilderHashCore, select::CircuitBuilderSelectHelpers};
use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::{
        target::{BoolTarget, Target},
        witness::{PartialWitness, Witness},
    },
    plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher},
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::{MerkleProof, MerkleProofBase, MerkleProofCore};

#[derive(Debug, Clone)]
pub struct VariableHeightMerkleProofGadget {
    pub root: HashOutTarget,
    pub value: HashOutTarget,
    pub index: Target,
    pub siblings: Vec<HashOutTarget>,
    pub height: Target,
    max_height: usize,
    has_witness_height: bool,
}
pub fn hash_merkle_leaves<F: RichField + Extendable<D>, const D: usize, H:AlgebraicHasher<F>>(
    builder: &mut CircuitBuilder<F, D>,
    leaves: &[HashOutTarget],
) -> HashOutTarget {
    // log2(leaves.len())
    let height = leaves.len().next_power_of_two().trailing_zeros() as usize;
    // ensure leaves.len() is a power of 2
    assert_eq!(
        1 << height,
        leaves.len(),
        "leaves.len() must be a power of 2"
    );
    let mut state = leaves.to_vec();
    for _ in 0..height {
        let mut next_state = vec![];
        for i in (0..state.len()).step_by(2) {
            let left = state[i];
            let right = if i + 1 < state.len() {
                state[i + 1]
            } else {
                state[i]
            };
            next_state
                .push(builder.hash_n_to_hash_no_pad::<H>([left.elements, right.elements].concat()));
        }
        state = next_state;
    }
    state[0]
}
impl VariableHeightMerkleProofGadget {
    pub fn add_virtual_to_full<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        max_height: usize,
        input_height_target: Option<Target>,
    ) -> Self {
        let index = builder.add_virtual_target();
        let value = builder.add_virtual_hash();
        let siblings = (0..max_height)
            .map(|_| builder.add_virtual_hash())
            .collect::<Vec<_>>();

        let has_witness_height = input_height_target.is_none();
        let height = match input_height_target {
            Some(v) => v,
            None => builder.add_virtual_target(),
        };

        let root = Self::compute_root::<H,F,D>(
            builder, 
            index, 
            value, 
            &siblings, 
            height
        );
        
        Self {
            root,
            value,
            index,
            siblings,
            max_height,
            has_witness_height,
            height,
        }
    }
    pub fn compute_root<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        index: Target,
        value: HashOutTarget,
        siblings: &[HashOutTarget],
        height_target: Target,
    ) -> HashOutTarget {
        let height = siblings.len();
        builder.range_check(index, height);
        let index_bits = builder.split_le(index, height);

        Self::compute_root_bits::<H, F, D>(builder, &index_bits, value, siblings, height_target)
    }
    pub fn compute_root_bits<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        builder: &mut CircuitBuilder<F, D>,
        index_bits: &[BoolTarget],
        value: HashOutTarget,
        siblings: &[HashOutTarget],
        height: Target,
    ) -> HashOutTarget {
        //let zero = builder.zero();
        let mut state: HashOutTarget = value;
        //debug_assert_eq!(state.elements.len(), NUM_HASH_OUT_ELEMENTS);

        let one_target = builder.one();
        let mut remaining_levels = height;
        let mut is_remaining_levels_zero = builder.is_zero(remaining_levels);
        //let mut is_remaining_levels_not_zero = builder.not(is_remaining_levels_zero);

        for (&bit, &sibling) in index_bits.iter().zip(siblings) {

            let left = builder.select_hash(bit, sibling, state);
            let right = builder.select_hash(bit, state, sibling);
            let proposed_state = builder.hash_two_to_one::<H>(
                left,
                right,
            );

            state = builder.select_hash(is_remaining_levels_zero, state, proposed_state);

            remaining_levels = builder.sub(remaining_levels, one_target);
            let is_cur_remaining_levels_zero = builder.is_zero(remaining_levels);
            is_remaining_levels_zero = builder.or(
                is_remaining_levels_zero,
                is_cur_remaining_levels_zero,
            );
        }
        state
    }
    pub fn set_witness_generic<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        index: F,
        value: QHashOut<F>,
        siblings: &[QHashOut<F>],
    ) {
            witness.set_target(self.index, index);
            witness.set_hash_target(self.value, value.0);
            if self.has_witness_height {
                witness.set_target(self.height, F::from_canonical_usize(siblings.len()));
            }
            for i in 0..siblings.len() {
                witness.set_hash_target(self.siblings[i], siblings[i].0);
            }
            for i in siblings.len()..self.max_height {
                witness.set_hash_target(self.siblings[i], HashOut::ZERO);
            }
    }
    pub fn set_witness<F: RichField>(
        &self,
        witness: &mut PartialWitness<F>,
        index: F,
        value: QHashOut<F>,
        siblings: &[QHashOut<F>],
    ) {
        self.set_witness_generic(witness, index, value, siblings);
    }
    pub fn set_witness_proof<F: RichField>(
        &self,
        witness: &mut PartialWitness<F>,
        input: &MerkleProof<F>,
    ) {
        self.set_witness(witness, input.index, input.value, &input.siblings);
    }
    pub fn set_witness_base_proof<F: RichField>(
        &self,
        witness: &mut PartialWitness<F>,
        input: &MerkleProofBase<F>,
    ) {
        self.set_witness(witness, input.index, input.value, &input.siblings);
    }
    pub fn set_witness_core_proof_q_generic<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        input: &MerkleProofCore<QHashOut<F>>,
    ) {
        self.set_witness_generic(
            witness,
            F::from_noncanonical_u64(input.index),
            input.value,
            &input.siblings,
        );
    }
    pub fn set_witness_core_proof_q<F: RichField>(
        &self,
        witness: &mut PartialWitness<F>,
        input: &MerkleProofCore<QHashOut<F>>,
    ) {
        self.set_witness(
            witness,
            F::from_noncanonical_u64(input.index),
            input.value,
            &input.siblings,
        );
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::types::{Field, PrimeField64};
    use plonky2::hash::poseidon::PoseidonHash;
    use plonky2::iop::target::Target;
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData};
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::plonk::proof::ProofWithPublicInputs;
    use qed_core::data::qhashout::QHashOut;
    use qed_crypto::hash::merkle::core::MerkleProofCore;
    use qed_crypto::hash::merkle::utils::simple_merkle_tree::SimpleMerkleTree;
    use qed_crypto::hash::traits::hasher::PoseidonHasher;

    use crate::hash::merkle::gadgets::variable_height_merkle_proof::VariableHeightMerkleProofGadget;


    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    struct TestVariableHeightMerkleProofCircuit {
        pub variable_merkle_proof_gadget: VariableHeightMerkleProofGadget,
        pub height: Target,
        pub circuit_data: CircuitData<F, C, D>,
    }

    impl TestVariableHeightMerkleProofCircuit {
        pub fn new(max_height: usize) -> Self {
            let config = CircuitConfig::standard_recursion_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);
            let height = builder.add_virtual_target();
            let variable_merkle_proof_gadget = VariableHeightMerkleProofGadget::add_virtual_to_full::<PoseidonHash, F, D>(
                &mut builder,
                max_height,
                Some(height),
            );
    
            builder.register_public_inputs(&variable_merkle_proof_gadget.root.elements);
            let circuit_data = builder.build::<C>();
            Self {
                variable_merkle_proof_gadget,
                height,
                circuit_data,
            }
        }
        pub fn prove(&self, height: usize, merkle_proof: &MerkleProofCore<QHashOut<F>>) -> anyhow::Result<ProofWithPublicInputs<F,C,D>> {
            let mut pw = PartialWitness::new();
            pw.set_target(self.height, F::from_canonical_usize(height));
            self.variable_merkle_proof_gadget.set_witness_core_proof_q_generic(
                &mut pw,
                merkle_proof,
            );
            self.circuit_data.prove(pw)
        }
    }

    pub fn generate_random_merkle_proofs_at_height(height: usize, count: usize) -> Vec<MerkleProofCore<QHashOut<F>>> {
        let max_leaf_index_mask = (1u64<<(height as u64))-1u64;
        let mut tree = SimpleMerkleTree::<PoseidonHasher, QHashOut<F>>::new(height as u8);
        // add some random leaves
        for _ in 0..(count*2) {
            let rand_index = QHashOut::<F>::rand().0.elements[0].to_canonical_u64()&max_leaf_index_mask;
            tree.set_leaf(rand_index, QHashOut::rand());
        }
        (0..count).map(|_|{
            let rand_index = QHashOut::<F>::rand().0.elements[0].to_canonical_u64()&max_leaf_index_mask;
            tree.set_leaf(rand_index, QHashOut::rand());
            tree.get_leaf(rand_index)
        }).collect()
    }

    #[test]
    fn test_variable_merkle_proof_circuit() {
        let circuit = TestVariableHeightMerkleProofCircuit::new(32);
        let merkle_proofs = (1..32).map(|level| {
            generate_random_merkle_proofs_at_height(level, 2)
        }).flatten().collect::<Vec<_>>();

        for mp in merkle_proofs.iter() {
            let proof = circuit.prove(mp.siblings.len(), mp).unwrap();
            assert_eq!(proof.public_inputs, mp.root.0.elements.to_vec(), "roots should match proof");
            circuit.circuit_data.verify(proof).unwrap();
        }
    }

    #[test]
    #[should_panic]
    fn test_variable_merkle_proof_circuit_incorrect_height() {
        let circuit = TestVariableHeightMerkleProofCircuit::new(32);
        let merkle_proofs = (1..32).map(|level| {
            generate_random_merkle_proofs_at_height(level, 2)
        }).flatten().collect::<Vec<_>>();

        for mp in merkle_proofs.iter() {
            let proof = circuit.prove(mp.siblings.len()+1, mp).unwrap();
            assert_eq!(proof.public_inputs, mp.root.0.elements.to_vec(), "roots should match proof");
            circuit.circuit_data.verify(proof).unwrap();
        }
    }
}
