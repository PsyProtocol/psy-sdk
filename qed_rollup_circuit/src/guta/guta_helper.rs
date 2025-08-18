use async_trait::async_trait;
use plonky2::{
    hash::hash_types::HashOut,
    plonk::{
        circuit_data::CommonCircuitData,
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_common_circuit::circuits::{lookalikes::get_end_cap_type_e_common_data, traits::qstandard::{QStandardCircuit, QStandardCircuitProvableWithProofStoreAndRefLibraryAsync}};
use qed_core::{
    config::network_constants::{CHECKPOINT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, GUTA_CIRCUIT_WHITELIST_TREE_HEIGHT, REALM_USER_TREE_HEIGHT},
    data::qhashout::QHashOut,
    job::{
        id::{ProvingJobCircuitType, QProvingJobDataID},
        traits::QProofStoreReaderAsync,
    },
};
use qed_crypto::{
    common::{
        circuit_library::{CircuitInfoLibrary, CircuitInfoLibraryBuilder},
        worker::{QNextGenWorkerGenericInfo, QNextGenWorkerGenericProverAsyncMut},
    },
    hash::{
        merkle::{core::MerkleProofCore, utils::simple_merkle_tree::SimpleMerkleTree},
        traits::hasher::{FieldQHasher, MerkleZeroHasher},
    },
};



use super::circuits::{
    guta_no_change::GUTANoChangeCircuit, only_register_users::GUTAOnlyRegisterUsersCircuit, verify_guta_and_register_users::GUTAVerifyGUTARegisterUsersCircuit, verify_guta_to_cap::GUTAVerifyGUTAToCapCircuit, verify_left_end_cap_right_guta::GUTAVerifyLeftEndCapRightGUTACircuit, verify_left_guta_right_end_cap::GUTAVerifyLeftGUTARightEndCapCircuit, verify_single_end_cap::GUTAVerifySingleEndCapCircuit, verify_two_end_cap::GUTAVerifyTwoEndCapCircuit, verify_two_guta::GUTAVerifyTwoGUTACircuit
};

#[derive(Debug)]
pub struct QEDGUTACircuitManager<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub end_cap_fingerprint: QHashOut<C::F>,

    pub verify_single_end_cap: GUTAVerifySingleEndCapCircuit<C, D>,
    pub verify_two_end_cap: GUTAVerifyTwoEndCapCircuit<C, D>,
    pub verify_two_guta: GUTAVerifyTwoGUTACircuit<C, D>,
    pub verify_left_guta_right_end_cap: GUTAVerifyLeftGUTARightEndCapCircuit<C, D>,

    pub verify_left_end_cap_right_guta: GUTAVerifyLeftEndCapRightGUTACircuit<C, D>,
    pub verify_guta_register_users: GUTAVerifyGUTARegisterUsersCircuit<C, D>,
    pub verify_guta_to_cap: GUTAVerifyGUTAToCapCircuit<C, D>,
    pub only_register_users: GUTAOnlyRegisterUsersCircuit<C, D>,
    pub no_change: GUTANoChangeCircuit<C, D>,

    pub guta_circuit_whitelist_root: QHashOut<C::F>,
    pub public_key: QHashOut<C::F>,
    pub verify_single_end_cap_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_two_end_cap_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_two_guta_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_left_guta_right_end_cap_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_left_end_cap_right_guta_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_guta_register_users_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_guta_to_cap_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub only_register_users_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub no_change_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> QEDGUTACircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub fn new_with_library<T: CircuitInfoLibrary<C,D>>(library: &T, public_key: QHashOut<C::F>) -> Self {
        let end_cap_common = get_end_cap_type_e_common_data::<C,D>();
        Self::new_with_config(
            &end_cap_common,
            library.get_verifier_data_cap_height(ProvingJobCircuitType::UserEndCap).unwrap(),
            library.get_fingerprint(ProvingJobCircuitType::UserEndCap).unwrap(),
            public_key,
        )
    }
    pub fn new_with_config(
        end_cap_proof_common_data: &CommonCircuitData<C::F, D>,
        end_cap_proof_verifier_data_cap_height: usize,
        known_end_cap_fingerprint: QHashOut<C::F>,
        public_key: QHashOut<C::F>,
    ) -> Self {
        let verify_single_end_cap = GUTAVerifySingleEndCapCircuit::<C, D>::new(
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint,
        );

        let verify_two_end_cap = GUTAVerifyTwoEndCapCircuit::<C, D>::new(
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint,
        );

        let guta_proof_common_data: &CommonCircuitData<C::F, D> =
            verify_two_end_cap.get_common_circuit_data_ref();
        let guta_proof_verifier_data_cap_height: usize = verify_single_end_cap
            .get_verifier_config_ref()
            .constants_sigmas_cap
            .height();

        let verify_two_guta = GUTAVerifyTwoGUTACircuit::<C, D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
        );

        let verify_left_guta_right_end_cap = GUTAVerifyLeftGUTARightEndCapCircuit::<C, D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint,
        );

        let verify_left_end_cap_right_guta = GUTAVerifyLeftEndCapRightGUTACircuit::<C, D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint,
        );

        let verify_guta_register_users = GUTAVerifyGUTARegisterUsersCircuit::<C, D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
            32,
            GLOBAL_USER_TREE_HEIGHT as usize,
        );

        let verify_guta_to_cap = GUTAVerifyGUTAToCapCircuit::<C, D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
        );

        let only_register_users =
            GUTAOnlyRegisterUsersCircuit::<C, D>::new(64, GLOBAL_USER_TREE_HEIGHT as usize);

        let no_change = GUTANoChangeCircuit::<C,D>::new(CHECKPOINT_TREE_HEIGHT as usize);


        let mut guta_circuit_whitelist_proofs =
            SimpleMerkleTree::<C::Hasher, QHashOut<C::F>>::gen_fast_tree_inclusion_proofs(
                GUTA_CIRCUIT_WHITELIST_TREE_HEIGHT,
                &[
                    verify_single_end_cap.get_fingerprint(),
                    verify_two_end_cap.get_fingerprint(),
                    verify_two_guta.get_fingerprint(),
                    verify_left_guta_right_end_cap.get_fingerprint(),
                    verify_left_end_cap_right_guta.get_fingerprint(),
                    verify_guta_register_users.get_fingerprint(),
                    verify_guta_to_cap.get_fingerprint(),
                    only_register_users.get_fingerprint(),
                    no_change.get_fingerprint(),
                ],
            )
            .unwrap();
        guta_circuit_whitelist_proofs.reverse();

        let verify_single_end_cap_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_two_end_cap_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_two_guta_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_left_guta_right_end_cap_whitelist_proof =
            guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_left_end_cap_right_guta_whitelist_proof =
            guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_guta_register_users_whitelist_proof =
            guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_guta_to_cap_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let only_register_users_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let no_change_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();

        Self {
            end_cap_fingerprint: known_end_cap_fingerprint,

            verify_single_end_cap,
            verify_two_end_cap,
            verify_two_guta,
            verify_left_guta_right_end_cap,

            verify_left_end_cap_right_guta,
            verify_guta_register_users,
            verify_guta_to_cap,
            only_register_users,
            no_change,

            guta_circuit_whitelist_root: verify_two_guta_whitelist_proof.root,
            public_key,
            verify_single_end_cap_whitelist_proof,
            verify_two_end_cap_whitelist_proof,
            verify_two_guta_whitelist_proof,
            verify_left_guta_right_end_cap_whitelist_proof,
            verify_left_end_cap_right_guta_whitelist_proof,
            verify_guta_register_users_whitelist_proof,
            verify_guta_to_cap_whitelist_proof,
            only_register_users_whitelist_proof,
            no_change_whitelist_proof,
        }
    }

    pub fn print_common_config(&self) {
        println!(
            "\n\n\n\n================================\n[verify_single_end_cap.common]:\n{:?}",
            self.verify_single_end_cap.get_common_circuit_data_ref()
        );
        println!(
            "================================\n[verify_two_end_cap.common]:\n{:?}",
            self.verify_two_end_cap.get_common_circuit_data_ref()
        );
        println!(
            "================================\n[verify_two_guta.common]:\n{:?}",
            self.verify_two_guta.get_common_circuit_data_ref()
        );
        println!(
            "================================\n[verify_left_guta_right_end_cap.common]:\n{:?}",
            self.verify_left_guta_right_end_cap
                .get_common_circuit_data_ref()
        );
        println!(
            "================================\n[verify_left_end_cap_right_guta.common]:\n{:?}",
            self.verify_left_end_cap_right_guta
                .get_common_circuit_data_ref()
        );
        println!(
            "================================\n[verify_guta_register_users.common]:\n{:?}",
            self.verify_guta_register_users
                .get_common_circuit_data_ref()
        );
        println!(
            "================================\n[verify_guta_to_cap.common]:\n{:?}",
            self.verify_guta_to_cap.get_common_circuit_data_ref()
        );
        println!(
            "================================\n[only_register_users.common]:\n{:?}",
            self.only_register_users.get_common_circuit_data_ref()
        );
        println!(
            "================================\n[no_change.common]:\n{:?}",
            self.no_change.get_common_circuit_data_ref()
        );
        println!("===============================\n\n\n\n");
    }
    pub fn register_library<T: CircuitInfoLibraryBuilder<C::F>>(&self, library: &mut T) {
        library.register_circuit(
            ProvingJobCircuitType::GUTASingleEndCap.into(),
            self.verify_single_end_cap.get_fingerprint(),
            self.verify_single_end_cap.get_verifier_config_ref().into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTATwoEndCap.into(),
            self.verify_two_end_cap.get_fingerprint(),
            self.verify_two_end_cap.get_verifier_config_ref().into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTATwoGUTA.into(),
            self.verify_two_guta.get_fingerprint(),
            self.verify_two_guta.get_verifier_config_ref().into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTALeftGUTARightEndCap.into(),
            self.verify_left_guta_right_end_cap.get_fingerprint(),
            self.verify_left_guta_right_end_cap
                .get_verifier_config_ref()
                .into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA.into(),
            self.verify_left_end_cap_right_guta.get_fingerprint(),
            self.verify_left_end_cap_right_guta
                .get_verifier_config_ref()
                .into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTARegisterUsers.into(),
            self.verify_guta_register_users.get_fingerprint(),
            self.verify_guta_register_users
                .get_verifier_config_ref()
                .into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTAVerifyToCap.into(),
            self.verify_guta_to_cap.get_fingerprint(),
            self.verify_guta_to_cap.get_verifier_config_ref().into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTAOnlyRegisterUsers.into(),
            self.only_register_users.get_fingerprint(),
            self.only_register_users.get_verifier_config_ref().into(),
        );
        library.register_circuit(
            ProvingJobCircuitType::GUTANoChange.into(),
            self.no_change.get_fingerprint(),
            self.no_change.get_verifier_config_ref().into(),
        );

        let all_group = [
            ProvingJobCircuitType::GUTASingleEndCap,
            ProvingJobCircuitType::GUTATwoEndCap,
            ProvingJobCircuitType::GUTATwoGUTA,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA,
            ProvingJobCircuitType::GUTARegisterUsers,
            ProvingJobCircuitType::GUTAOnlyRegisterUsers,
            ProvingJobCircuitType::GUTAVerifyToCap,
            ProvingJobCircuitType::GUTANoChange,
        ];

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTASingleEndCap,
            self.verify_single_end_cap_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTATwoEndCap,
            self.verify_two_end_cap_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTATwoGUTA,
            self.verify_two_guta_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap,
            self.verify_left_guta_right_end_cap_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA,
            self.verify_left_end_cap_right_guta_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTARegisterUsers,
            self.verify_guta_register_users_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTAOnlyRegisterUsers,
            self.only_register_users_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTAVerifyToCap,
            self.verify_guta_to_cap_whitelist_proof.clone(),
        );

        library.add_inclusion_proof(
            &all_group,
            ProvingJobCircuitType::GUTANoChange,
            self.no_change_whitelist_proof.clone(),
        );
    }
}

impl<
        C: GenericConfig<D> + 'static,
        const D: usize,
    > QNextGenWorkerGenericInfo for QEDGUTACircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    fn can_process_job(&self, job_id: QProvingJobDataID) -> bool {
        match job_id.circuit_type {
            ProvingJobCircuitType::GUTASingleEndCap => true,
            ProvingJobCircuitType::GUTATwoEndCap => true,
            ProvingJobCircuitType::GUTATwoGUTA => true,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap => true,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA => true,
            ProvingJobCircuitType::GUTARegisterUsers => true,
            ProvingJobCircuitType::GUTAOnlyRegisterUsers => true,
            ProvingJobCircuitType::GUTAVerifyToCap => true,
            ProvingJobCircuitType::GUTANoChange => true,
            _ => false
        }
    }

}
#[async_trait]
impl<
        S: QProofStoreReaderAsync + Send + Sync,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        C: GenericConfig<D> + 'static,
        const D: usize,
    > QNextGenWorkerGenericProverAsyncMut<S, L, C, D> for QEDGUTACircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>
{
    async fn worker_prove_mut_async(
        &self,
        store: &S,
        library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match job_id.circuit_type {
            ProvingJobCircuitType::GUTASingleEndCap => self.verify_single_end_cap.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTATwoEndCap => self.verify_two_end_cap.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTATwoGUTA => self.verify_two_guta.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap => self.verify_left_guta_right_end_cap.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA => self.verify_left_end_cap_right_guta.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTARegisterUsers => self.verify_guta_register_users.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTAOnlyRegisterUsers => self.only_register_users.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTAVerifyToCap => self.verify_guta_to_cap.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            ProvingJobCircuitType::GUTANoChange => self.no_change.prove_with_proof_store_async(store, library, job_id, self.public_key).await,
            _ => anyhow::bail!("unsupported circuit: {:?}", job_id.circuit_type),
        }
    }
}
#[cfg(test)]
mod tests {
    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use qed_common_circuit::circuits::lookalikes::custom::get_lookalike_custom;
    use qed_core::{data::qhashout::QHashOut, job::id::QCircuitCommonGatesType};



    use super::QEDGUTACircuitManager;

    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;
    #[test]
    fn check_fingerprints() {
        let end_cap_info = get_lookalike_custom::<C,D>(
            QCircuitCommonGatesType::E,
            12,
            4
        );

        let mgr = QEDGUTACircuitManager::<C, D>::new_with_config(
            &end_cap_info.common,
            end_cap_info
                .verifier_only
                .constants_sigmas_cap
                .height(),
            QHashOut::rand(),
            QHashOut::rand(),  // public_key for testing
        );


        assert_eq!(
            mgr.verify_single_end_cap.fingerprint,
            mgr.verify_single_end_cap_whitelist_proof.value,
        );
        assert_eq!(
            mgr.verify_two_end_cap.fingerprint,
            mgr.verify_two_end_cap_whitelist_proof.value,
        );
        assert_eq!(
            mgr.verify_two_guta.fingerprint,
            mgr.verify_two_guta_whitelist_proof.value,
        );
        assert_eq!(
            mgr.verify_left_guta_right_end_cap.fingerprint,
            mgr.verify_left_guta_right_end_cap_whitelist_proof.value,
        );

        assert_eq!(
            mgr.verify_left_end_cap_right_guta.fingerprint,
            mgr.verify_left_end_cap_right_guta_whitelist_proof.value,
        );
        assert_eq!(
            mgr.verify_guta_register_users.fingerprint,
            mgr.verify_guta_register_users_whitelist_proof.value,
        );
        assert_eq!(
            mgr.verify_guta_to_cap.fingerprint,
            mgr.verify_guta_to_cap_whitelist_proof.value,
        );
        assert_eq!(
            mgr.only_register_users.fingerprint,
            mgr.only_register_users_whitelist_proof.value,
        );
        assert_eq!(
            mgr.no_change.fingerprint,
            mgr.no_change_whitelist_proof.value,
        );
    }
}
