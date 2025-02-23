use async_trait::async_trait;
use plonky2::{
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
    circuits::traits::qstandard::{
        QStandardCircuit, QStandardCircuitProvableWithProofStoreAndRefLibraryAsync,
    },
    proof_minifier::pm_core::get_circuit_fingerprint_generic,
};
use qed_core::{
    data::qhashout::QHashOut,
    job::{
        id::{ProvingJobCircuitType, QProvingJobDataID},
        traits::QProofStoreReaderAsync,
    },
};
use qed_crypto::{
    common::circuit_library::CircuitInfoLibrary,
    hash::{
        merkle::{
            core::MerkleProofCore,
            treeprover::{
                data::CircuitInputWithDependencies, AggStateTransition,
                TPAltCircuitFingerprintConfig,
            },
        },
        traits::hasher::MerkleZeroHasher,
    },
};
use qed_data::{
    guta::header::GlobalUserTreeAggregatorHeader,
    protocol::circuit_inputs::agg_part_1::QCAggUserRegistartionDeployContractsGUTAInput,
};

use crate::coordinator::gadgets::verify_agg_user_registration_deploy_guta::VerifyAggUserRegistartionDeployContractsGUTAGadget;

#[derive(Debug)]
pub struct VerifyAggUserRegistartionDeployContractsGUTACircuit<C: GenericConfig<D>, const D: usize>
{
    pub verifier_gadget: VerifyAggUserRegistartionDeployContractsGUTAGadget<D>,

    pub circuit_data: CircuitData<C::F, C, D>,
    pub fingerprint: QHashOut<C::F>,
}

impl<C: GenericConfig<D>, const D: usize> VerifyAggUserRegistartionDeployContractsGUTACircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new(
        user_reg_proof_common_data: &CommonCircuitData<C::F, D>,
        user_reg_transition_circuit_config: &TPAltCircuitFingerprintConfig<C::F>,

        deploy_contracts_proof_common_data: &CommonCircuitData<C::F, D>,
        deploy_contracts_transition_circuit_config: &TPAltCircuitFingerprintConfig<C::F>,

        guta_proof_common_data: &CommonCircuitData<C::F, D>,
        guta_verifier_data_cap_height: usize,
        guta_circuit_whitelist_root: QHashOut<C::F>,
    ) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let verifier_gadget =
            VerifyAggUserRegistartionDeployContractsGUTAGadget::<D>::add_virtual_to::<C, C::F>(
                &mut builder,
                user_reg_proof_common_data,
                user_reg_transition_circuit_config,
                deploy_contracts_proof_common_data,
                deploy_contracts_transition_circuit_config,
                guta_proof_common_data,
                guta_verifier_data_cap_height,
                guta_circuit_whitelist_root,
            );
        let state_transition_hash = verifier_gadget
            .header
            .get_combined_hash::<C::Hasher, C::F, D>(&mut builder);

        builder.register_public_inputs(&state_transition_hash.elements);
        let circuit_data = builder.build::<C>();

        let fingerprint = QHashOut(get_circuit_fingerprint_generic(&circuit_data.verifier_only));

        Self {
            circuit_data,
            fingerprint,
            verifier_gadget,
        }
    }

    pub fn prove_base(
        &self,
        register_users_state_transition: &AggStateTransition<C::F>,
        register_users_proof: &ProofWithPublicInputs<C::F, C, D>,
        register_users_verifier_data: &VerifierOnlyCircuitData<C, D>,

        deploy_contracts_state_transition: &AggStateTransition<C::F>,
        deploy_contracts_proof: &ProofWithPublicInputs<C::F, C, D>,
        deploy_contracts_verifier_data: &VerifierOnlyCircuitData<C, D>,

        guta_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        guta_proof_header: &GlobalUserTreeAggregatorHeader<C::F>,
        guta_proof: &ProofWithPublicInputs<C::F, C, D>,
        guta_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        self.verifier_gadget.set_witness_params(
            &mut pw,
            register_users_state_transition,
            register_users_proof,
            register_users_verifier_data,
            deploy_contracts_state_transition,
            deploy_contracts_proof,
            deploy_contracts_verifier_data,
            guta_whitelist_merkle_proof,
            guta_proof_header,
            guta_proof,
            guta_verifier_data,
        )?;

        self.circuit_data.prove(pw)
    }
}

impl<C: GenericConfig<D>, const D: usize> QStandardCircuit<C, D>
    for VerifyAggUserRegistartionDeployContractsGUTACircuit<C, D>
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
    for VerifyAggUserRegistartionDeployContractsGUTACircuit<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    async fn prove_with_proof_store_async(
        &self,
        store: &S,
        library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let r: CircuitInputWithDependencies<QCAggUserRegistartionDeployContractsGUTAInput<C::F>> =
            bincode::deserialize(&store.get_bytes_by_id(job_id.get_input_witness_id()).await?)
                .map_err(|e| anyhow::anyhow!(e))?;

        println!("deps: {:?}",r.dependencies);

        let user_registration_proof = store.get_proof_by_id(r.dependencies[0]).await?;
        println!("urp");

        let deploy_contracts_proof = store.get_proof_by_id(r.dependencies[1]).await?;
        println!("dcp");
        let guta_proof = store.get_proof_by_id(r.dependencies[2]).await?;
        println!("guta");
        
        let user_registration_type = r.dependencies[0].circuit_type;
        let deploy_contracts_type = r.dependencies[1].circuit_type;
        let guta_type = r.dependencies[2].circuit_type;

        let user_registration_verifier_data = library.get_verifier_data(user_registration_type)?;
        let deploy_contracts_verifier_data = library.get_verifier_data(deploy_contracts_type)?;
        let guta_verifier_data = library.get_verifier_data(guta_type)?;

        let guta_inclusion_proof =
            library.get_group_inclusion_proof(ProvingJobCircuitType::GUTATwoGUTA, guta_type)?;

        let result = self.prove_base(
            &r.input.register_users_state_transition,
            &user_registration_proof,
            &user_registration_verifier_data,
            &r.input.deploy_contracts_state_transition,
            &deploy_contracts_proof,
            &deploy_contracts_verifier_data,
            &guta_inclusion_proof,
            &r.input.guta_proof_header,
            &guta_proof,
            &guta_verifier_data,
        )?;

        Ok(result)
    }
}
