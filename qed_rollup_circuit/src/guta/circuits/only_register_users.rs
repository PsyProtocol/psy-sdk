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
    builder::pad_circuit::CircuitBuilderQEDCommonGates, circuits::traits::qstandard::{ QStandardCircuit, QStandardCircuitProvableWithProofStoreAndRefLibraryAsync}, proof_minifier::
        pm_core::get_circuit_fingerprint_generic
};
use qed_core::{config::network_constants::{DEFAULT_USER_STATE_TREE_ROOT_U64, GLOBAL_USER_TREE_HEIGHT, REALM_USER_TREE_HEIGHT}, data::qhashout::QHashOut, job::{id::{ProvingJobCircuitType, QProvingJobDataID}, traits::QProofStoreReaderAsync}};
use qed_crypto::{common::circuit_library::CircuitInfoLibrary, hash::traits::hasher::MerkleZeroHasher};
use qed_data::guta::proof_input::{GUTAOnlyRegisterUsersInput, GUTARegisterUserFullInput};

use crate::guta::gadgets::guta_only_register_users_gadget::GUTAOnlyRegisterUsersGadget;

#[derive(Debug)]
pub struct GUTAOnlyRegisterUsersCircuit<C: GenericConfig<D>, const D: usize>
{
    register_batch_gadget: GUTAOnlyRegisterUsersGadget,
    guta_circuit_whitelist: HashOutTarget,
    checkpoint_tree_root: HashOutTarget,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D>, const D: usize> GUTAOnlyRegisterUsersCircuit<C, D>
where
    C::Hasher:AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> {
        pub fn new(
            max_users: usize,
            global_user_tree_realm_height: usize,
            
        ) -> Self {


        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
       
        let guta_circuit_whitelist = builder.add_virtual_hash();
        let checkpoint_tree_root = builder.add_virtual_hash();
        

        let default_user_state_tree_root = QHashOut::from_values(
            DEFAULT_USER_STATE_TREE_ROOT_U64[0],
            DEFAULT_USER_STATE_TREE_ROOT_U64[1],
            DEFAULT_USER_STATE_TREE_ROOT_U64[2],
            DEFAULT_USER_STATE_TREE_ROOT_U64[3],
        );


        let register_batch_gadget = GUTAOnlyRegisterUsersGadget::add_virtual_to::<C::Hasher, C::F, D>(
            &mut builder,
            guta_circuit_whitelist,
            checkpoint_tree_root,
            global_user_tree_realm_height,
            GLOBAL_USER_TREE_HEIGHT as usize,
            default_user_state_tree_root,
            max_users,
        );

        let public_inputs_hash = register_batch_gadget.new_guta_header.to_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&public_inputs_hash.elements);
        builder.add_qed_type_c_common_gates();
        //builder.add_gate_to_gate_set(GateRef::new(ConstantGate::new(builder.config.num_constants)));
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(
            &circuit_data.verifier_only,
        ));

        Self {
            register_batch_gadget,
            guta_circuit_whitelist,
            checkpoint_tree_root,
            
            circuit_data,
            fingerprint,
        }
    }
    
    pub fn prove_base(
        &self,
        guta_circuit_whitelist_root: QHashOut<C::F>,
        checkpoint_tree_root: QHashOut<C::F>,
        guta_register_user_inputs: &[GUTARegisterUserFullInput<C::F>],
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();


        let default_user_state_tree_root = QHashOut::from_values(
            DEFAULT_USER_STATE_TREE_ROOT_U64[0],
            DEFAULT_USER_STATE_TREE_ROOT_U64[1],
            DEFAULT_USER_STATE_TREE_ROOT_U64[2],
            DEFAULT_USER_STATE_TREE_ROOT_U64[3],
        );

        pw.set_hash_target(
            self.guta_circuit_whitelist,
            guta_circuit_whitelist_root.0,
        )?;
        pw.set_hash_target(
            self.checkpoint_tree_root,
            checkpoint_tree_root.0,
        )?;


        self.register_batch_gadget.set_witness_params::<C::Hasher, C::F, D>(
            &mut pw,
            guta_register_user_inputs,
            default_user_state_tree_root,
        )?;

        self.circuit_data.prove(pw)
    }
}


impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for GUTAOnlyRegisterUsersCircuit<C, D>
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
    for GUTAOnlyRegisterUsersCircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let r: GUTAOnlyRegisterUsersInput<C::F> =
            bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?)
                .map_err(|e| anyhow::anyhow!(e))?;


        let guta_whitelist_root: QHashOut<C::F> =
            library.get_group_inclusion_proof(ProvingJobCircuitType::GUTATwoGUTA, ProvingJobCircuitType::GUTATwoGUTA)?.root;
        

        let result = self.prove_base(
            guta_whitelist_root,
            r.checkpoint_tree_root,
            &r.guta_register_user_inputs,
        )?;

        Ok(result)
    }
}
