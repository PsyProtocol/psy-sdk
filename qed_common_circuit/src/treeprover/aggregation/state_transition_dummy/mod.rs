use async_trait::async_trait;
use plonky2::{
    hash::hash_types::HashOutTarget,
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_core::{config::network_constants::get_default_worker_public_key, data::qhashout::QHashOut, job::{id::QProvingJobDataID, traits::{QProofStoreReaderAsync, QProofStoreReaderSync}}};
use qed_crypto::{common::circuit_library::CircuitInfoLibrary, hash::merkle::treeprover::DummyAggStateTransition};

use crate::{
    builder::{
        hash::core::CircuitBuilderHashCore,
        pad_circuit::{pad_circuit_degree, CircuitBuilderQEDCommonGates},
    },
    circuits::traits::qstandard::{
        provable::QStandardCircuitProvable, QStandardCircuit, QStandardCircuitProvableWithProofStoreAndRefLibraryAsync, QStandardCircuitProvableWithProofStoreSync
    },
    proof_minifier::pm_core::get_circuit_fingerprint_generic,
};

#[derive(Debug)]
pub struct AggStateTransitionDummyCircuit<C: GenericConfig<D>, const D: usize>
{
    pub state_transition_hash: HashOutTarget,
    pub allowed_circuit_hashes_root: HashOutTarget,
    pub worker_public_key: HashOutTarget,

    // end circuit targets
    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}
impl<C: GenericConfig<D>, const D: usize> AggStateTransitionDummyCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let state_transition_hash = builder.add_virtual_hash();
        let allowed_circuit_hashes_root = builder.add_virtual_hash();
        let worker_public_key = builder.add_virtual_hash();
        let commitment = worker_public_key;

        let transition =
            builder.hash_two_to_one::<C::Hasher>(state_transition_hash, state_transition_hash);

        builder.register_public_inputs(&commitment.elements);
        builder.register_public_inputs(&worker_public_key.elements);
        builder.register_public_inputs(&allowed_circuit_hashes_root.elements);
        builder.register_public_inputs(&transition.elements);

        builder.add_qed_type_d_common_gates();
        pad_circuit_degree::<C::F, D>(&mut builder, 12);
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            state_transition_hash,
            allowed_circuit_hashes_root,
            worker_public_key,
            circuit_data,
            fingerprint,
        }
    }
    pub fn prove_base(
        &self,
        worker_public_key: QHashOut<C::F>,
        state_transition_hash: QHashOut<C::F>,
        allowed_circuit_hashes_root: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();
        //tracing::info!("agg_fingerprint: {}", agg_fingerprint.to_string());
        //tracing::info!("leaf_fingerprint: {}", leaf_fingerprint.to_string());
        pw.set_hash_target(self.worker_public_key, worker_public_key.0)?;
        pw.set_hash_target(self.state_transition_hash, state_transition_hash.0)?;
        pw.set_hash_target(
            self.allowed_circuit_hashes_root,
            allowed_circuit_hashes_root.0,
        )?;
        self.circuit_data.prove(pw)
    }
}

/*
impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for AggStateTransitionDummyCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
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
*/

impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for AggStateTransitionDummyCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
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
    QStandardCircuitProvable<DummyAggStateTransition<C::F>, C, D>
    for AggStateTransitionDummyCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    fn prove_standard(
        &self,
        input: &DummyAggStateTransition<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.prove_base(
            get_default_worker_public_key(),
            input.state_transition_hash,
            input.allowed_circuit_hashes_root,
        )
    }
}

impl<S: QProofStoreReaderSync, C: GenericConfig<D>, const D: usize>
    QStandardCircuitProvableWithProofStoreSync<S, DummyAggStateTransition<C::F>, C, D>
    for AggStateTransitionDummyCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    fn prove_with_proof_store_sync(
        &self,
        _store: &S,
        input: &DummyAggStateTransition<C::F>,
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
    for AggStateTransitionDummyCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        _library: &L,
        job_id: QProvingJobDataID,
        worker_public_key: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let r: DummyAggStateTransition<C::F> =
            bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?)
                .map_err(|e| anyhow::anyhow!(e))?;
        self.prove_base(
            worker_public_key,
            r.state_transition_hash,
            r.allowed_circuit_hashes_root,
        )
    }
}
