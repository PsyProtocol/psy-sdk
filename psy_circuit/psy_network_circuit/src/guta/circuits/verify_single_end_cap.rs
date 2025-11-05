use async_trait::async_trait;
use plonky2::{
    field::types::Field,
    hash::hash_types::{HashOut, HashOutTarget},
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common::{
    data::qhashout::QHashOut,
    job::{id::QProvingJobDataID, traits::QProofStoreReaderAsync},
};
use psy_common_circuit::{
    builder::{comparison::CircuitBuilderComparison, hash::core::CircuitBuilderHashCore, pad_circuit::pad_circuit_degree},
    circuits::traits::qstandard::{QStandardCircuit, QStandardCircuitProvableWithProofStoreAndRefLibraryAsync},
    proof_minifier::pm_core::get_circuit_fingerprint_generic,
    traits::{ToTargets, WitnessValueFor},
};
use psy_crypto::{
    common::circuit_library::CircuitInfoLibrary,
    hash::{merkle::treeprover::data::CircuitInputWithDependencies, traits::hasher::MerkleZeroHasher},
};
use psy_data::{guta::proof_input::VerifySingleEndCapInput, qdata::pm_jobs_completed_stats::PMJobsCompletedStats};

use crate::{
    gadgets::qdata::pm_jobs_completed_stats::PMJobsCompletedStatsGadget,
    guta::gadgets::{helpers::ToGUTAHeader, verify_end_cap::VerifyEndCapProofGadget},
};

#[derive(Debug)]
pub struct GUTAVerifySingleEndCapCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub guta_circuit_whitelist_root_hash: HashOutTarget,
    pub a_end_cap_gadget: VerifyEndCapProofGadget<D>,
    pub worker_public_key: HashOutTarget,
    pub pm_jobs_completed: PMJobsCompletedStatsGadget,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> GUTAVerifySingleEndCapCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new(
        end_cap_proof_common_data: &CommonCircuitData<C::F, D>,
        end_cap_proof_verifier_data_cap_height: usize,
        known_end_cap_fingerprint: QHashOut<C::F>,
    ) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let known_end_cap_fingerprint_hash = builder.constant_qhash(known_end_cap_fingerprint);

        let guta_circuit_whitelist_root_hash = builder.add_virtual_hash();

        let a_end_cap_gadget = VerifyEndCapProofGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint_hash,
        );

        let a_end_cap_guta_header = a_end_cap_gadget.get_guta_header::<C::Hasher, C::F>(&mut builder, guta_circuit_whitelist_root_hash);

        tracing::debug!("📊 a_end_cap_guta_header: {:?}", a_end_cap_guta_header);

        let worker_public_key = builder.add_virtual_hash();

        // builder.assert_non_zero_hash(worker_public_key);

        let zero_hash = builder.constant_hash(HashOut::ZERO);
        let commitment = builder.hash_two_to_one::<C::Hasher>(zero_hash, zero_hash);

        let one = builder.one();
        let pm_jobs_completed = PMJobsCompletedStatsGadget::new_gutas(&mut builder, one);

        let public_inputs_hash = a_end_cap_guta_header.to_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&commitment.elements);
        builder.register_public_inputs(&worker_public_key.elements);
        builder.register_public_inputs(&pm_jobs_completed.to_targets());
        builder.register_public_inputs(&public_inputs_hash.elements);

        pad_circuit_degree(&mut builder, 12);
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            guta_circuit_whitelist_root_hash,
            a_end_cap_gadget,
            worker_public_key,
            pm_jobs_completed,
            circuit_data,
            fingerprint,
        }
    }

    pub fn prove_base(
        &self,
        worker_public_key: QHashOut<C::F>,
        input: &VerifySingleEndCapInput<C::F>,
        child_a_proof: &ProofWithPublicInputs<C::F, C, D>,
        end_cap_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();
        pw.set_hash_target(self.guta_circuit_whitelist_root_hash, input.guta_circuit_whitelist.0)?;
        pw.set_hash_target(self.worker_public_key, worker_public_key.0)?;

        let jobs_completed_stats = PMJobsCompletedStats::new_gutas(C::F::ONE);
        self.pm_jobs_completed.set_witness(&mut pw, &jobs_completed_stats)?;

        self.a_end_cap_gadget.set_witness(
            &mut pw,
            &input.get_end_result_a(),
            &input.a_end_cap.guta_stats,
            &input.a_end_cap.checkpoint_historical_merkle_proof,
            child_a_proof,
            end_cap_verifier_data,
        )?;

        self.circuit_data.prove(pw)
    }
}

impl<C: GenericConfig<D> + 'static, const D: usize> QStandardCircuit<C, D> for GUTAVerifySingleEndCapCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
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

#[async_trait]
impl<S: QProofStoreReaderAsync + Send + Sync, L: CircuitInfoLibrary<C, D> + Send + Sync, C: GenericConfig<D> + 'static, const D: usize>
    QStandardCircuitProvableWithProofStoreAndRefLibraryAsync<S, L, C, D> for GUTAVerifySingleEndCapCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        library: &L,
        job_id: QProvingJobDataID,
        worker_public_key: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let r: CircuitInputWithDependencies<VerifySingleEndCapInput<C::F>> =
            bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?).map_err(|e| anyhow::anyhow!(e))?;
        tracing::debug!("GUTAVerifySingleEndCapCircuitInput: {}", serde_json::to_string_pretty(&r)?);

        if r.dependencies.len() != 1 {
            anyhow::bail!("invalid dependency count in two end guta input");
        }

        let child_a_proof = store.get_proof_by_id(r.dependencies[0]).await?;

        let dep_a_type = r.dependencies[0].circuit_type;

        let child_a_verifier_data = library.get_verifier_data(dep_a_type)?;

        let result = self.prove_base(worker_public_key, &r.input, &child_a_proof, &child_a_verifier_data)?;

        Ok(result)
    }
}
