use plonky2::{
    gates::{constant::ConstantGate, gate::GateRef}, hash::hash_types::{HashOut, HashOutTarget}, iop::
        witness::{PartialWitness, WitnessWrite}, plonk::{
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
use qed_core::{data::qhashout::QHashOut, job::{id::QProvingJobDataID, traits::QProofStoreReaderAsync}, utils::debug_timer::DebugTimer};
use qed_crypto::{common::circuit_library::CircuitInfoLibrary, hash::{merkle::treeprover::data::CircuitInputWithDependencies, traits::hasher::MerkleZeroHasher}};
use qed_data::guta::proof_input::VerifyTwoEndCapCircuitInput;

use crate::guta::gadgets::{helpers::ToGUTAHeader, two_nca_state_transition::TwoNCAStateTransitionGadget, verify_end_cap::VerifyEndCapProofGadget};
use async_trait::async_trait;

#[derive(Debug)]
pub struct GUTAVerifyTwoEndCapCircuit<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub guta_circuit_whitelist_root_hash: HashOutTarget,
    pub a_end_cap_gadget: VerifyEndCapProofGadget<D>,
    pub b_end_cap_gadget: VerifyEndCapProofGadget<D>,
    pub nca_state_transition_gadget: TwoNCAStateTransitionGadget,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D>+ 'static, const D: usize> GUTAVerifyTwoEndCapCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> {
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

        let b_end_cap_gadget = VerifyEndCapProofGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint_hash,
        );
        

        let a_end_cap_guta_header = a_end_cap_gadget.get_guta_header::<C::Hasher, C::F>(
            &mut builder,
            guta_circuit_whitelist_root_hash,
        );

        let b_end_cap_guta_header = b_end_cap_gadget.get_guta_header::<C::Hasher, C::F>(
            &mut builder,
            guta_circuit_whitelist_root_hash,
        );

        let nca_state_transition_gadget = TwoNCAStateTransitionGadget::add_virtual_to::<C::Hasher, C::F, D>(
            &mut builder,
            a_end_cap_guta_header,
            b_end_cap_guta_header,
        );

        let public_inputs_hash = nca_state_transition_gadget.new_guta_header.to_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&public_inputs_hash.elements);
        builder.add_gate_to_gate_set(GateRef::new(ConstantGate::new(builder.config.num_constants)));
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));

        Self {
            guta_circuit_whitelist_root_hash,
            a_end_cap_gadget,
            b_end_cap_gadget,
            nca_state_transition_gadget,
            circuit_data,
            fingerprint,
        }
    }
    
    pub fn prove_base(
        &self,
        input: &VerifyTwoEndCapCircuitInput<C::F>,
        child_a_proof: &ProofWithPublicInputs<C::F, C, D>,
        child_b_proof: &ProofWithPublicInputs<C::F, C, D>,
        end_cap_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        pw.set_hash_target(self.guta_circuit_whitelist_root_hash, input.guta_circuit_whitelist.0)?;

        self.a_end_cap_gadget.set_witness(
            &mut pw,
            &input.get_end_result_a(),
            &input.a_end_cap.guta_stats,
            &input.a_end_cap.checkpoint_historical_merkle_proof,
            child_a_proof,
            end_cap_verifier_data
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

        let mut dbgt = DebugTimer::new("prove end cap two");
        dbgt.lap("start");

        let result = self.circuit_data.prove(pw);

        dbgt.lap("finished");
        result
    }
}


impl<C: GenericConfig<D> + 'static, const D: usize> QStandardCircuit<C, D>
    for GUTAVerifyTwoEndCapCircuit<C, D>
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
impl <
    S: QProofStoreReaderAsync + Send + Sync,
    L: CircuitInfoLibrary<C,D> + Send + Sync,
    C: GenericConfig<D>+ 'static,
    const D: usize,
> QStandardCircuitProvableWithProofStoreAndRefLibraryAsync<S, L, C, D> for GUTAVerifyTwoEndCapCircuit<C,D> 
where
C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {

        let r: CircuitInputWithDependencies<VerifyTwoEndCapCircuitInput<C::F>> = bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?).map_err(|e| anyhow::anyhow!(e))?; 
        if r.dependencies.len() != 2 {
            anyhow::bail!("invalid dependency count in two end cap input");
        }

        let proof_a = store.get_proof_by_id(r.dependencies[0]).await?;
        let proof_b = store.get_proof_by_id(r.dependencies[1]).await?;

        let dep_0_type = r.dependencies[0].circuit_type;
        let vd = library.get_verifier_data(dep_0_type)?;
        let result = self.prove_base(
            &r.input,
            &proof_a,
            &proof_b,
            &vd,
        )?;

        Ok(result)


    }
}

