use plonky2::{
    hash::hash_types::{HashOut, HashOutTarget},
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
    field::types::Field,
};
use qed_common_circuit::{
    builder::{
        hash::core::CircuitBuilderHashCore,
        pad_circuit::{pad_circuit_degree, CircuitBuilderQEDCommonGates},
    },
    circuits::traits::qstandard::{
        provable::QStandardCircuitProvable, QStandardCircuit,
        QStandardCircuitProvableWithProofStoreSync,
    },
    proof_minifier::pm_core::get_circuit_fingerprint_generic,
};
use qed_core::{data::qhashout::QHashOut, job::traits::QProofStoreReaderSync};
use qed_crypto::{hash::traits::hasher::MerkleZeroHasher, signature::zk::wallet::PRIVATE_KEY_CONSTANTS};
use qed_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

use crate::dpn::vm::compile::QEDContractFunctionBuilderGadget;

#[derive(Debug)]
pub struct DapenContractFunctionCircuit<C: GenericConfig<D>, const D: usize> {
    pub inputs: Vec<Target>,
    pub fn_builder_gadget: QEDContractFunctionBuilderGadget,

    // end circuit targets
    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,

    // end circuit data
    pub fn_def: DPNFunctionCircuitDefinition,
}

impl<C: GenericConfig<D>, const D: usize> Clone for DapenContractFunctionCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn clone(&self) -> Self {
        Self::new(
            &self.fn_def,
            self.fn_builder_gadget
                .state_reader
                .contract_state_tree_height,
            self.fn_builder_gadget
                .state_reader
                .session_proof_tree_height,
            self.fn_builder_gadget.state_reader.force_four_align,
        )
    }
}

impl<C: GenericConfig<D>, const D: usize> DapenContractFunctionCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new(
        //coset_gate: &GateRef<C::F, D>,
        fn_def: &DPNFunctionCircuitDefinition,
        contract_state_tree_height: usize,
        session_proof_tree_height: usize,
        force_four_align: bool,
    ) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let inputs = builder.add_virtual_targets(fn_def.circuit_inputs.len());
        let fn_builder_gadget =
            QEDContractFunctionBuilderGadget::add_virtual_to::<C::Hasher, C::F, D>(
                &mut builder,
                fn_def,
                contract_state_tree_height,
                session_proof_tree_height,
                inputs.clone(),
                force_four_align,
            );

        let inner_public_inputs_hash = fn_builder_gadget
            .tx_ctx_header
            .to_hash::<C::Hasher, C::F, D>(&mut builder);
        let public_inputs_hash = builder.hash_two_to_one::<C::Hasher>(
            fn_builder_gadget.session_proof_tree_root,
            inner_public_inputs_hash,
        );

        builder.register_public_inputs(&public_inputs_hash.elements);
        //builder.add_qed_type_a_common_gates(Some(coset_gate.clone()));
        builder.add_qed_type_b_common_gates();
        pad_circuit_degree::<C::F, D>(&mut builder, 11);

        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            inputs,
            fn_builder_gadget,
            circuit_data,
            fingerprint,
            fn_def: fn_def.clone(),
        }
    }
    pub fn prove_base(
        &self,
        cfc_input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        pw.set_target_arr(&self.inputs, &cfc_input.inputs)?;

        pw.set_hash_target(
            self.fn_builder_gadget.session_proof_tree_root,
            cfc_input.session_proof_tree_root.0,
        )?;

        self.fn_builder_gadget
            .tx_ctx_header
            .set_witness(&mut pw, &cfc_input.tx_input_ctx)?;
        self.fn_builder_gadget
            .state_reader
            .set_witness(&mut pw, cfc_input, &self.fn_def);

        self.circuit_data.prove(pw)
    }
}

impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for DapenContractFunctionCircuit<C, D>
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        self.fingerprint
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        &self.circuit_data.verifier_only
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        &self.circuit_data.common
    }
}
impl<C: GenericConfig<D>, const D: usize>
    QStandardCircuitProvable<DapenContractFunctionCircuitInput<C::F>, C, D>
    for DapenContractFunctionCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn prove_standard(
        &self,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.prove_base(input)
    }
}

impl<S: QProofStoreReaderSync, C: GenericConfig<D>, const D: usize>
    QStandardCircuitProvableWithProofStoreSync<S, DapenContractFunctionCircuitInput<C::F>, C, D>
    for DapenContractFunctionCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn prove_with_proof_store_sync(
        &self,
        _store: &S,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.prove_standard(input)
    }
}

#[derive(Debug)]
pub struct DapenContractFunctionCircuitV2<C: GenericConfig<D>, const D: usize> {
    pub private_key: HashOutTarget,
    pub inputs: Vec<Target>,
    pub fn_builder_gadget: QEDContractFunctionBuilderGadget,
    pub sig_hash: HashOutTarget,

    // end circuit targets
    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,

    // end circuit data
    pub fn_def: DPNFunctionCircuitDefinition,
}

impl<C: GenericConfig<D>, const D: usize> Clone for DapenContractFunctionCircuitV2<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn clone(&self) -> Self {
        Self::new(
            &self.fn_def,
            self.fn_builder_gadget
                .state_reader
                .contract_state_tree_height,
            self.fn_builder_gadget
                .state_reader
                .session_proof_tree_height,
            self.fn_builder_gadget.state_reader.force_four_align,
        )
    }
}

impl<C: GenericConfig<D>, const D: usize> DapenContractFunctionCircuitV2<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new(
        //coset_gate: &GateRef<C::F, D>,
        fn_def: &DPNFunctionCircuitDefinition,
        contract_state_tree_height: usize,
        session_proof_tree_height: usize,
        force_four_align: bool,
    ) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let private_key = builder.add_virtual_hash();
        let inputs = builder.add_virtual_targets(fn_def.circuit_inputs.len());
        let sig_hash = builder.add_virtual_hash();
        let fn_builder_gadget =
            QEDContractFunctionBuilderGadget::add_virtual_to::<C::Hasher, C::F, D>(
                &mut builder,
                fn_def,
                contract_state_tree_height,
                session_proof_tree_height,
                inputs.clone(),
                force_four_align,
            );

        // enforce user contract state tree root is not change
        let start_contract_state_tree_root = fn_builder_gadget
            .tx_ctx_header
            .transaction_call_start_ctx
            .start_contract_state_tree_root;
        let end_contract_state_tree_root = fn_builder_gadget
            .tx_ctx_header
            .transaction_end_ctx
            .end_contract_state_tree_root;
        builder.connect_hashes(start_contract_state_tree_root, end_contract_state_tree_root);

        let private_key_constants = PRIVATE_KEY_CONSTANTS
            .iter()
            .map(|c| builder.constant(C::F::from_canonical_u64(*c)))
            .collect::<Vec<_>>();
        let public_key_param = builder.hash_n_to_hash_no_pad::<C::Hasher>(vec![
            private_key_constants[0],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[19],
            private_key.elements[1],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[3],
            private_key_constants[4],
            private_key_constants[5],
            private_key_constants[6],
            private_key.elements[0],
            private_key_constants[7],
            private_key.elements[2],
            private_key_constants[8],
            private_key_constants[9],
            private_key_constants[10],
            private_key_constants[11],
            private_key_constants[12],
            private_key.elements[3],
            private_key_constants[13],
            private_key_constants[14],
            private_key_constants[15],
            private_key_constants[16],
            private_key_constants[17],
            private_key_constants[18],
        ]);
        let public_inputs_hash = builder.hash_two_to_one::<C::Hasher>(sig_hash, public_key_param);

        builder.register_public_inputs(&public_inputs_hash.elements);

        builder.add_qed_type_b_common_gates();
        pad_circuit_degree::<C::F, D>(&mut builder, 11);

        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            private_key,
            inputs,
            sig_hash,
            fn_builder_gadget,
            circuit_data,
            fingerprint,
            fn_def: fn_def.clone(),
        }
    }
    pub fn prove_base(
        &self,
        private_key: QHashOut<C::F>,
        cfc_input: &DapenContractFunctionCircuitInput<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        pw.set_hash_target(self.private_key, private_key.0)?;

        pw.set_target_arr(&self.inputs, &cfc_input.inputs)?;

        pw.set_hash_target(
            self.fn_builder_gadget.session_proof_tree_root,
            cfc_input.session_proof_tree_root.0,
        )?;
        pw.set_hash_target(self.sig_hash, sig_hash.0)?;

        self.fn_builder_gadget
            .tx_ctx_header
            .set_witness(&mut pw, &cfc_input.tx_input_ctx)?;
        self.fn_builder_gadget
            .state_reader
            .set_witness(&mut pw, cfc_input, &self.fn_def);

        self.circuit_data.prove(pw)
    }
}

impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for DapenContractFunctionCircuitV2<C, D>
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        self.fingerprint
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        &self.circuit_data.verifier_only
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        &self.circuit_data.common
    }
}
