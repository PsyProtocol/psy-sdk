use async_trait::async_trait;
use plonky2::{
    gates::{constant::ConstantGate, gate::GateRef}, hash::hash_types::HashOut, iop::
        witness::PartialWitness, plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    }
};
use qed_common_circuit::{
    builder::hash::core::CircuitBuilderHashCore, circuits::traits::qstandard::{ QStandardCircuit, QStandardCircuitProvableWithProofStoreAndRefLibraryAsync}, proof_minifier::
        pm_core::get_circuit_fingerprint_generic
};
use qed_core::{data::qhashout::QHashOut, job::{id::QProvingJobDataID, traits::QProofStoreReaderAsync}};
use qed_crypto::{common::circuit_library::CircuitInfoLibrary, hash::{merkle::treeprover::data::CircuitInputWithDependencies, traits::hasher::MerkleZeroHasher}};
use qed_data::guta::proof_input::{VerifyLeftGUTARightEndCapInput, VerifyLeftGUTARightEndCapInputSimple};

use crate::guta::gadgets::{helpers::ToGUTAHeader, two_nca_state_transition::TwoNCAStateTransitionGadget, verify_end_cap::VerifyEndCapProofGadget, verify_guta_proof::VerifyGUTAProofGadget};

#[derive(Debug)]
pub struct GUTAVerifyLeftGUTARightEndCapCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub a_guta_gadget: VerifyGUTAProofGadget<D>,
    pub b_end_cap_gadget: VerifyEndCapProofGadget<D>,
    pub nca_state_transition_gadget: TwoNCAStateTransitionGadget,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> GUTAVerifyLeftGUTARightEndCapCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> {
        pub fn new(
            guta_proof_common_data: &CommonCircuitData<C::F, D>,
            guta_proof_verifier_data_cap_height: usize,
            end_cap_proof_common_data: &CommonCircuitData<C::F, D>,
            end_cap_proof_verifier_data_cap_height: usize,
            known_end_cap_fingerprint: QHashOut<C::F>,
        ) -> Self {

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let known_end_cap_fingerprint_hash = builder.constant_qhash(known_end_cap_fingerprint);


        let a_guta_gadget = VerifyGUTAProofGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
        );

        let b_end_cap_gadget = VerifyEndCapProofGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint_hash,
        );
        

        let a_guta_header = a_guta_gadget.get_guta_header::<C::Hasher, C::F>(
            &mut builder,
            a_guta_gadget.guta_proof_header_gadget.guta_circuit_whitelist,
        );

        let b_guta_header = b_end_cap_gadget.get_guta_header::<C::Hasher, C::F>(
            &mut builder,
            a_guta_gadget.guta_proof_header_gadget.guta_circuit_whitelist,
        );
        

        let nca_state_transition_gadget = TwoNCAStateTransitionGadget::add_virtual_to::<C::Hasher, C::F, D>(
            &mut builder,
            a_guta_header,
            b_guta_header,
        );

        let public_inputs_hash = nca_state_transition_gadget.new_guta_header.to_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&public_inputs_hash.elements);

        builder.add_gate_to_gate_set(GateRef::new(ConstantGate::new(builder.config.num_constants)));
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));
// test
        Self {
            a_guta_gadget,
            b_end_cap_gadget,
            nca_state_transition_gadget,
            circuit_data,
            fingerprint,
        }
    }
    
    pub fn prove_base(
        &self,
        input: &VerifyLeftGUTARightEndCapInput<C::F>,
        child_a_proof: &ProofWithPublicInputs<C::F, C, D>,
        child_a_verifier_data: &VerifierOnlyCircuitData<C, D>,
        child_b_proof: &ProofWithPublicInputs<C::F, C, D>,
        end_cap_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        self.a_guta_gadget.set_witness(
            &mut pw,
            &input.guta_inclusion_proof_a,

            &input.get_guta_header_a(),
            child_a_proof,
            child_a_verifier_data
        )?;
        self.b_end_cap_gadget.set_witness(
            &mut pw,
            &input.get_end_result_b(),
            &input.b_end_cap.guta_stats,
            &input.b_end_cap.checkpoint_historical_merkle_proof,
            child_b_proof,
            end_cap_verifier_data
        )?;

        self.nca_state_transition_gadget.set_witness_partial(
            &mut pw, 
            &input.nca_proof
        )?;

        self.circuit_data.prove(pw)
    }
}


impl<C: GenericConfig<D> + 'static, const D: usize> QStandardCircuit<C, D>
    for GUTAVerifyLeftGUTARightEndCapCircuit<C, D>
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


#[async_trait]
impl<
        S: QProofStoreReaderAsync + Send + Sync,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        C: GenericConfig<D> + 'static,
        const D: usize,
    > QStandardCircuitProvableWithProofStoreAndRefLibraryAsync<S, L, C, D>
    for GUTAVerifyLeftGUTARightEndCapCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let r: CircuitInputWithDependencies<VerifyLeftGUTARightEndCapInputSimple<C::F>> =
            bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?)
                .map_err(|e| anyhow::anyhow!(e))?;
        if r.dependencies.len() != 1 {
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

        

        let result = self.prove_base(
            &VerifyLeftGUTARightEndCapInput { checkpoint_tree_root: r.input.checkpoint_tree_root, stats_a:r.input.stats_a, b_end_cap: r.input.b_end_cap, nca_proof: r.input.nca_proof, guta_inclusion_proof_a},
            &child_a_proof,
            &child_a_verifier_data,
            &child_b_proof,
            &child_b_verifier_data
        )?;

        Ok(result)
    }
}
