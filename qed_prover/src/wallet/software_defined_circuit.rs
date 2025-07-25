use plonky2::{
    gates::gate::GateRef,
    hash::hash_types::HashOut,
    plonk::{
        circuit_data::{CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_common_circuit::{
    circuits::traits::qstandard::QStandardCircuit, proof_minifier::pm_chain::QEDProofMinifierChain,
    u32::gates::comparison::ComparisonGate,
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
use qed_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

use crate::dpn::circuits::cfc::DapenContractFunctionCircuitV2;

#[derive(Debug)]
pub struct SoftwareDefinedCircuit<C: GenericConfig<D>, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub minifier_chain: QEDProofMinifierChain<D, C::F, C>,
    pub inner_circuit: DapenContractFunctionCircuitV2<C, D>,
}

impl<C: GenericConfig<D>, const D: usize> Clone for SoftwareDefinedCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn clone(&self) -> Self {
        Self::new(
            &self.inner_circuit.fn_def,
            self.inner_circuit
                .fn_builder_gadget
                .state_reader
                .contract_state_tree_height,
            self.inner_circuit
                .fn_builder_gadget
                .state_reader
                .session_proof_tree_height,
            self.inner_circuit
                .fn_builder_gadget
                .state_reader
                .force_four_align,
        )
    }
}

impl<C: GenericConfig<D>, const D: usize> SoftwareDefinedCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new(
        fn_def: &DPNFunctionCircuitDefinition,
        contract_state_tree_height: usize,
        session_proof_tree_height: usize,
        force_four_align: bool,
    ) -> Self {
        let inner_circuit = DapenContractFunctionCircuitV2::new(
            fn_def,
            contract_state_tree_height,
            session_proof_tree_height,
            force_four_align,
        );

        let added_gates_for_minifier = [GateRef::new(ComparisonGate::new(32, 16))];

        let minifier_chain = QEDProofMinifierChain::<D, C::F, C>::new_add_gates(
            &inner_circuit.circuit_data.verifier_only,
            &inner_circuit.circuit_data.common,
            2,
            Some(&added_gates_for_minifier),
        );

        Self {
            inner_circuit,
            minifier_chain,
        }
    }
    pub fn prove(
        &self,
        cfc_input: &DapenContractFunctionCircuitInput<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let inner_proof = self.inner_circuit.prove_base(cfc_input, sig_hash)?;
        self.minifier_chain.prove(&inner_proof)
    }
}

impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D> for SoftwareDefinedCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        QHashOut(self.minifier_chain.get_fingerprint())
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        self.minifier_chain.get_verifier_data()
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        self.minifier_chain.get_common_data()
    }
}
