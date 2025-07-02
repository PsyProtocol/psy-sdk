use async_trait::async_trait;
use plonky2::{
    gates::{constant::ConstantGate, gate::GateRef},
    hash::hash_types::HashOut,
    iop::witness::PartialWitness,
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_common_circuit::{
    builder::pad_circuit::pad_circuit_degree, circuits::traits::qstandard::{
        QStandardCircuit,
        QStandardCircuitProvableWithProofStoreAndRefLibraryAsync,
    }, proof_minifier::pm_core::get_circuit_fingerprint_generic
};
use qed_core::{
    data::qhashout::QHashOut,
    job::{id::QProvingJobDataID, traits::QProofStoreReaderAsync},
};
use qed_crypto::{common::circuit_library::CircuitInfoLibrary, hash::{
    merkle::treeprover::data::CircuitInputWithDependencies, traits::hasher::MerkleZeroHasher,
}};
use qed_data::guta::proof_input::{
    VerifyTwoGUTAProofGadgetStandardInput, VerifyTwoGUTAProofGadgetStandardInputSimple,
};

use crate::guta::gadgets::{
    helpers::ToGUTAHeader, two_nca_state_transition::TwoNCAStateTransitionGadget,
    verify_guta_proof::VerifyGUTAProofGadget,
};

#[derive(Debug)]
pub struct GUTAVerifyTwoGUTACircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub a_guta_gadget: VerifyGUTAProofGadget<D>,
    pub b_guta_gadget: VerifyGUTAProofGadget<D>,
    pub nca_state_transition_gadget: TwoNCAStateTransitionGadget,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> GUTAVerifyTwoGUTACircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new(
        guta_proof_common_data: &CommonCircuitData<C::F, D>,
        guta_proof_verifier_data_cap_height: usize,
    ) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let a_guta_gadget = VerifyGUTAProofGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
        );

        let b_guta_gadget = VerifyGUTAProofGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
        );

        let a_guta_header = a_guta_gadget.get_guta_header::<C::Hasher, C::F>(
            &mut builder,
            a_guta_gadget
                .guta_proof_header_gadget
                .guta_circuit_whitelist,
            //a_guta_gadget.guta_whitelist_merkle_proof.root,
        );

        let b_guta_header = b_guta_gadget.get_guta_header::<C::Hasher, C::F>(
            &mut builder,
            b_guta_gadget
                .guta_proof_header_gadget
                .guta_circuit_whitelist,
        );

        let nca_state_transition_gadget = TwoNCAStateTransitionGadget::add_virtual_to::<
            C::Hasher,
            C::F,
            D,
        >(&mut builder, a_guta_header, b_guta_header);

        eprintln!("DEBUGPRINT[663]: verify_two_guta.rs:93: nca_state_transition_gadget.new_guta_header={:#?}", nca_state_transition_gadget.new_guta_header);
        let public_inputs_hash = nca_state_transition_gadget
            .new_guta_header
            .to_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&public_inputs_hash.elements);

        builder.add_gate_to_gate_set(GateRef::new(ConstantGate::new(
            builder.config.num_constants,
        )));
        pad_circuit_degree(&mut builder, 13);
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            a_guta_gadget,
            b_guta_gadget,
            nca_state_transition_gadget,
            circuit_data,
            fingerprint,
        }
    }

    pub fn prove_base(
        &self,
        input: &VerifyTwoGUTAProofGadgetStandardInput<C::F>,
        child_a_proof: &ProofWithPublicInputs<C::F, C, D>,
        child_a_verifier_data: &VerifierOnlyCircuitData<C, D>,
        child_b_proof: &ProofWithPublicInputs<C::F, C, D>,
        child_b_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        self.a_guta_gadget.set_witness(
            &mut pw,
            &input.guta_inclusion_proof_a,
            &input.get_guta_header_a(),
            child_a_proof,
            child_a_verifier_data,
        )?;
        self.b_guta_gadget.set_witness(
            &mut pw,
            &input.guta_inclusion_proof_b,
            &input.get_guta_header_b(),
            child_b_proof,
            child_b_verifier_data,
        )?;

        self.nca_state_transition_gadget
            .set_witness_partial(&mut pw, &input.nca_proof)?;

        self.circuit_data.prove(pw)
    }
}

impl<C: GenericConfig<D> + 'static, const D: usize> QStandardCircuit<C, D>
    for GUTAVerifyTwoGUTACircuit<C, D>
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
impl<
        S: QProofStoreReaderAsync + Send + Sync,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        C: GenericConfig<D> + 'static,
        const D: usize,
    > QStandardCircuitProvableWithProofStoreAndRefLibraryAsync<S, L, C, D>
    for GUTAVerifyTwoGUTACircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let r: CircuitInputWithDependencies<VerifyTwoGUTAProofGadgetStandardInputSimple<C::F>> =
            bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?)
                .map_err(|e| anyhow::anyhow!(e))?;
        eprintln!("DEBUGPRINT[665]: verify_two_guta.rs:186: r={}", serde_json::to_string_pretty(&r).unwrap());
        if r.dependencies.len() != 2 {
            anyhow::bail!("invalid dependency count in two end guta input");
        }

        let child_a_proof = store.get_proof_by_id(r.dependencies[0]).await?;
        let child_b_proof = store.get_proof_by_id(r.dependencies[1]).await?;

        let dep_a_type = r.dependencies[0].circuit_type;
        let dep_b_type = r.dependencies[1].circuit_type;

        let child_a_verifier_data = library.get_verifier_data(dep_a_type)?;
        let guta_inclusion_proof_a =
            library.get_group_inclusion_proof(job_id.circuit_type, dep_a_type)?;
        let child_b_verifier_data = library.get_verifier_data(dep_b_type)?;
        let guta_inclusion_proof_b =
            library.get_group_inclusion_proof(job_id.circuit_type, dep_b_type)?;

        let result = self.prove_base(
            &VerifyTwoGUTAProofGadgetStandardInput {
                checkpoint_tree_root: r.input.checkpoint_tree_root,
                b_checkpoint_tree_root: r.input.b_checkpoint_tree_root,
                stats_a: r.input.stats_a,
                stats_b: r.input.stats_b,
                nca_proof: r.input.nca_proof,
                guta_inclusion_proof_a,
                guta_inclusion_proof_b,
            },
            &child_a_proof,
            &child_a_verifier_data,
            &child_b_proof,
            &child_b_verifier_data,
        )?;
        eprintln!("DEBUGPRINT[666]: verify_two_guta.rs:219: result={}", serde_json::to_string_pretty(&result).unwrap());

        Ok(result)
    }
}
