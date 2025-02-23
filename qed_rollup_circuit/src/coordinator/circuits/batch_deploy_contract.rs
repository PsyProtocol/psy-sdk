use async_trait::async_trait;
use plonky2::{
    hash::hash_types::{HashOut, HashOutTarget}, iop::
        witness::{PartialWitness, WitnessWrite}, plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    }
};
use qed_common_circuit::{
    builder::{hash::core::CircuitBuilderHashCore, pad_circuit::{pad_circuit_degree, CircuitBuilderQEDCommonGates}}, circuits::traits::qstandard::{provable::QStandardCircuitProvable, QStandardCircuit, QStandardCircuitProvableWithProofStoreAndRefLibraryAsync, QStandardCircuitProvableWithProofStoreSync}, proof_minifier::
        pm_core::get_circuit_fingerprint_generic
};
use qed_core::{data::qhashout::QHashOut, job::{id::QProvingJobDataID, traits::{QProofStoreReaderAsync, QProofStoreReaderSync}}};
use qed_crypto::{common::circuit_library::CircuitInfoLibrary, hash::{merkle::spiderman::SpidermanUpdateProof, traits::hasher::MerkleZeroHasher}};
use qed_data::{protocol::circuit_inputs::deploy_contracts::QCBatchDeployContractsCircuitInput, qdata::contract::QEDContractLeaf};

use crate::coordinator::gadgets::deploy_contract::BatchDeployContractsGadget;

#[derive(Debug)]
pub struct BatchDeployContractsCircuit<C: GenericConfig<D>, const D: usize>
{
    pub deploy_contract_batch_gadget: BatchDeployContractsGadget,
    pub deploy_contract_circuit_whitelist: HashOutTarget,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D>, const D: usize> BatchDeployContractsCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> {
        pub fn new(
            contract_tree_height: usize,
            batch_sub_tree_height: usize,
        ) -> Self {


        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);


        let deploy_contract_circuit_whitelist = builder.add_virtual_hash();


        let deploy_contract_batch_gadget = BatchDeployContractsGadget::add_virtual_to::<C::Hasher, C::F, D>(
            &mut builder,
            contract_tree_height,
            batch_sub_tree_height,
        );

        let state_transition_hash = builder.hash_two_to_one::<C::Hasher>(
            deploy_contract_batch_gadget.spiderman_gadget.old_root,
            deploy_contract_batch_gadget.spiderman_gadget.new_root,
        );

        builder.register_public_inputs(&deploy_contract_circuit_whitelist.elements);
        builder.register_public_inputs(&state_transition_hash.elements);
        builder.add_qed_type_d_common_gates();
        pad_circuit_degree(&mut builder, 12);

        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));

        Self {
            deploy_contract_circuit_whitelist,
            deploy_contract_batch_gadget,
            circuit_data,
            fingerprint,
        }
    }
    
    pub fn prove_base(
        &self,
        deploy_contract_circuit_whitelist: QHashOut<C::F>,
        spiderman_append_proof: &SpidermanUpdateProof<QHashOut<C::F>>,
        contract_leaves: &[QEDContractLeaf<C::F>],

    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();
        pw.set_hash_target(self.deploy_contract_circuit_whitelist, deploy_contract_circuit_whitelist.0)?;
        self.deploy_contract_batch_gadget.set_witness_params(
            &mut pw,
            spiderman_append_proof,
            contract_leaves,
        )?;

        self.circuit_data.prove(pw)
    }
}


impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for BatchDeployContractsCircuit<C, D>
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
    QStandardCircuitProvable<QCBatchDeployContractsCircuitInput<C::F>, C, D> for BatchDeployContractsCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn prove_standard(
        &self,
        input: &QCBatchDeployContractsCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.prove_base(
            input.deploy_contract_circuit_whitelist,
            &input.spiderman_append_proof,
            &input.contract_leaves,
        )
    }
}

impl<S: QProofStoreReaderSync, C: GenericConfig<D>, const D: usize>
    QStandardCircuitProvableWithProofStoreSync<S, QCBatchDeployContractsCircuitInput<C::F>, C, D>
    for BatchDeployContractsCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn prove_with_proof_store_sync(
        &self,
        _store: &S,
        input: &QCBatchDeployContractsCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.prove_standard(input)
    }
}



#[async_trait]
impl<
        S: QProofStoreReaderAsync + Send + Sync,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        C: GenericConfig<D>,
        const D: usize,
    > QStandardCircuitProvableWithProofStoreAndRefLibraryAsync<S, L, C, D>
    for BatchDeployContractsCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        _library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let input: QCBatchDeployContractsCircuitInput<C::F> = bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?)
                .map_err(|e| anyhow::anyhow!(e))?;

        let result = self.prove_standard(&input)?;
        
        Ok(result)
    }
}
