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
    circuits::traits::qstandard::QStandardCircuit, proof_minifier::
        pm_core::get_circuit_fingerprint_generic
};
use qed_core::{config::network_constants::{DEFAULT_USER_STATE_TREE_ROOT_U64, GLOBAL_USER_TREE_HEIGHT}, data::qhashout::QHashOut};
use qed_crypto::hash::{merkle::core::MerkleProofCore, traits::hasher::MerkleZeroHasher};
use qed_data::guta::{header::GlobalUserTreeAggregatorHeader, proof_input::GUTARegisterUserFullInput};

use crate::guta::gadgets::guta_register_users_batch::GUTARegisterUsersBatchGadget;

#[derive(Debug)]
pub struct GUTAVerifyGUTARegisterUsersCircuit<C: GenericConfig<D>, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub register_batch_gadget: GUTARegisterUsersBatchGadget<D>,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D>, const D: usize> GUTAVerifyGUTARegisterUsersCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> {
        pub fn new(
            guta_proof_common_data: &CommonCircuitData<C::F, D>,
            guta_proof_verifier_data_cap_height: usize,
            max_users: usize,
        ) -> Self {

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);


        let default_user_state_tree_root = QHashOut::from_values(
            DEFAULT_USER_STATE_TREE_ROOT_U64[0],
            DEFAULT_USER_STATE_TREE_ROOT_U64[1],
            DEFAULT_USER_STATE_TREE_ROOT_U64[2],
            DEFAULT_USER_STATE_TREE_ROOT_U64[3],
        );


        let register_batch_gadget = GUTARegisterUsersBatchGadget::<D>::add_virtual_to::<C, C::F>(
            &mut builder,
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
            GLOBAL_USER_TREE_HEIGHT as usize,
            default_user_state_tree_root,
            max_users,
        );

        let public_inputs_hash = register_batch_gadget.new_guta_header.to_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&public_inputs_hash.elements);

        builder.add_gate_to_gate_set(GateRef::new(ConstantGate::new(builder.config.num_constants)));
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));

        Self {
            circuit_data,
            fingerprint,
            register_batch_gadget,
        }
    }
    
    pub fn prove_base(
        &self,
        guta_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        guta_proof_header: &GlobalUserTreeAggregatorHeader<C::F>,
        proof: &ProofWithPublicInputs<C::F, C, D>,
        verifier_data: &VerifierOnlyCircuitData<C, D>,
        top_line_siblings: &[QHashOut<C::F>],
        guta_register_user_inputs: &[GUTARegisterUserFullInput<C::F>],
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();


        let default_user_state_tree_root = QHashOut::from_values(
            DEFAULT_USER_STATE_TREE_ROOT_U64[0],
            DEFAULT_USER_STATE_TREE_ROOT_U64[1],
            DEFAULT_USER_STATE_TREE_ROOT_U64[2],
            DEFAULT_USER_STATE_TREE_ROOT_U64[3],
        );


        self.register_batch_gadget.set_witness_params(
            &mut pw,
            guta_whitelist_merkle_proof,
            guta_proof_header,
            proof,
            verifier_data,
            top_line_siblings,
            guta_register_user_inputs,
            default_user_state_tree_root
        )?;

        self.circuit_data.prove(pw)
    }
}


impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for GUTAVerifyGUTARegisterUsersCircuit<C, D>
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

