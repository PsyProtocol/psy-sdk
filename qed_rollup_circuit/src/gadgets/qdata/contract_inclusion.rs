use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::{target::Target, witness::Witness}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_common_circuit::{builder::core::CircuitBuilderHelpersCore, hash::merkle::gadgets::merkle_proof::MerkleProofGadget, traits::{CreatableTarget, CreatableWithHasherTarget, WitnessValueFor}};
use qed_core::{config::network_constants::{CONTRACT_FUNCTION_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT}, data::qhashout::QHashOut};
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::qdata::{contract::QEDContractLeaf, contract_inclusion::{QEDContractFunctionInclusionProof, QEDContractInclusionProof}};

use super::contract::QEDContractLeafGadget;





#[derive(Clone, Debug)]
pub struct QEDContractInclusionProofGadget {
    pub contract_leaf: QEDContractLeafGadget,
    pub contract_tree_merkle_proof: MerkleProofGadget,

    // computed
    pub contract_leaf_hash: HashOutTarget,
}

impl QEDContractInclusionProofGadget {
    pub fn add_virtual_to<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,

    ) -> Self {
        // START: create targets that require witness
        let contract_leaf = QEDContractLeafGadget::create_virtual::<F, D>(builder);
        let contract_tree_merkle_proof = MerkleProofGadget::add_virtual_to::<H, F, D>(builder, GLOBAL_CONTRACT_TREE_HEIGHT as usize);
        // END: create targets that require witness


        // START: setup computed targets
        let contract_leaf_hash = contract_leaf.to_hash::<H, F, D>(builder);

        // ensure that the computed contract_leaf_hash matches our merkle proof value
        builder.connect_hashes(
            contract_leaf_hash,
            contract_tree_merkle_proof.value
        );
        
        // END: setup computed targets



        Self {
            contract_tree_merkle_proof,
            contract_leaf,
            contract_leaf_hash,
        }

    }
    pub fn set_witness_params<F: RichField>(
        &self, 
        witness: &mut impl Witness<F>,
        contract_leaf: &QEDContractLeaf<F>,
        contract_tree_merkle_proof: &MerkleProofCore<QHashOut<F>>,
    ) {
        self.contract_leaf.set_witness(witness, contract_leaf);
        self.contract_tree_merkle_proof.set_witness_core_proof_q_generic(
            witness,
            contract_tree_merkle_proof,
        );
    }
    pub fn set_witness<F: RichField>(&self, witness: &mut impl Witness<F>, target: &QEDContractInclusionProof<F>) {
        self.set_witness_params(
            witness,
            &target.contract_leaf,
            &target.contract_tree_merkle_proof,
        );
    }
}
impl CreatableWithHasherTarget for QEDContractInclusionProofGadget {
    fn create_virtual_with_hasher<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        Self::add_virtual_to::<H, F, D>(builder)
    }
}

impl<F: RichField> WitnessValueFor<QEDContractInclusionProofGadget, F, true> for QEDContractInclusionProof<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDContractInclusionProofGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<QEDContractInclusionProofGadget, F, false> for QEDContractInclusionProof<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDContractInclusionProofGadget) {
        target.set_witness(witness, self);
    }
}








#[derive(Clone, Debug)]
pub struct QEDContractFunctionInclusionProofGadget {
    pub contract_inclusion_proof: QEDContractInclusionProofGadget,
    pub contract_function_merkle_proof: MerkleProofGadget,

    // computed
    pub function_verifier_fingerprint: HashOutTarget,
    pub method_id: Target,
    // outputs_and_inputs = (num_outputs<<32)|num_inputs
    pub outputs_and_inputs: Target,


    pub num_inputs: Target,
    pub num_outputs: Target,

    /* 
    */
    
}

impl QEDContractFunctionInclusionProofGadget {
    pub fn add_virtual_to<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,

    ) -> Self {
        // START: create targets that require witness
        let contract_inclusion_proof = QEDContractInclusionProofGadget::add_virtual_to::<H, F, D>(builder);

        let (contract_function_merkle_proof, cf_merkle_proof_index_bits) = MerkleProofGadget::add_virtual_to_get_index_bits::<H, F, D>(builder, CONTRACT_FUNCTION_TREE_HEIGHT as usize);
        // END: create targets that require witness


        // ensure that the index in the function tree is an EVEN number (least significant bit is 0)
        // this is because each function takes up two leaves (one for verifier key, one for metadata)
        builder.assert_zero(cf_merkle_proof_index_bits[0].target);

        // ensure the function_tree_root in the contract leaf matches our function tree merkle proof's root
        builder.connect_hashes(
            contract_inclusion_proof.contract_leaf.function_tree_root,
            contract_function_merkle_proof.root,
        );

        // START: setup computed targets

        // each function has two leaves, a left leaf and a right sibling:
        // **left** is the hash of the verifier key and **right** is [method_id, (num_outputs<<32)|num_inputs, 0, 0]
        
        let function_verifier_fingerprint = contract_function_merkle_proof.value;
        let method_id = contract_function_merkle_proof.siblings[0].elements[0];
        let outputs_and_inputs = contract_function_merkle_proof.siblings[0].elements[1];
        let (num_inputs, num_outputs) = builder.split_low_high_32bits(outputs_and_inputs);
        // END: setup computed targets



        Self {
            contract_inclusion_proof,
            contract_function_merkle_proof,
            function_verifier_fingerprint,
            method_id,
            outputs_and_inputs,
            num_inputs,
            num_outputs,
        }

    }
    pub fn set_witness_params<F: RichField>(
        &self, 
        witness: &mut impl Witness<F>,
        contract_inclusion_proof: &QEDContractInclusionProof<F>,
        contract_function_merkle_proof: &MerkleProofCore<QHashOut<F>>,
    ) {
        self.contract_inclusion_proof.set_witness(witness, contract_inclusion_proof);
        self.contract_function_merkle_proof.set_witness_core_proof_q_generic(
            witness,
            contract_function_merkle_proof,
        );
    }
    pub fn set_witness<F: RichField>(&self, witness: &mut impl Witness<F>, target: &QEDContractFunctionInclusionProof<F>) {
        self.set_witness_params(
            witness,
            &target.contract_inclusion_proof,
            &target.contract_function_merkle_proof,
        );
    }
}
impl CreatableWithHasherTarget for QEDContractFunctionInclusionProofGadget {
    fn create_virtual_with_hasher<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        Self::add_virtual_to::<H, F, D>(builder)
    }
}

impl<F: RichField> WitnessValueFor<QEDContractFunctionInclusionProofGadget, F, true> for QEDContractFunctionInclusionProof<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDContractFunctionInclusionProofGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<QEDContractFunctionInclusionProofGadget, F, false> for QEDContractFunctionInclusionProof<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDContractFunctionInclusionProofGadget) {
        target.set_witness(witness, self);
    }
}


