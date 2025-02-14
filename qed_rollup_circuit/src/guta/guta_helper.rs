use plonky2::{
    hash::hash_types::HashOut,
    plonk::{circuit_data::CommonCircuitData, config::{AlgebraicHasher, GenericConfig}},
};
use qed_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use qed_core::{
    config::network_constants::GUTA_CIRCUIT_WHITELIST_TREE_HEIGHT,
    data::qhashout::QHashOut,
};
use qed_crypto::hash::{
    merkle::{core::MerkleProofCore, utils::simple_merkle_tree::SimpleMerkleTree},
    traits::hasher::MerkleZeroHasher,
};

use super::circuits::{verify_guta_and_register_users::GUTAVerifyGUTARegisterUsersCircuit, verify_left_end_cap_right_guta::GUTAVerifyLeftEndCapRightGUTACircuit, verify_left_guta_right_end_cap::GUTAVerifyLeftGUTARightEndCapCircuit, verify_single_end_cap::GUTAVerifySingleEndCapCircuit, verify_two_end_cap::GUTAVerifyTwoEndCapCircuit, verify_two_guta::GUTAVerifyTwoGUTACircuit};

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
    
    pub guta_circuit_whitelist_root: QHashOut<C::F>,
    pub verify_single_end_cap_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_two_end_cap_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_two_guta_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_left_guta_right_end_cap_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_left_end_cap_right_guta_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub verify_guta_register_users_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> QEDGUTACircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub fn new_with_config(
        end_cap_proof_common_data: &CommonCircuitData<C::F, D>,
        end_cap_proof_verifier_data_cap_height: usize,
        known_end_cap_fingerprint: QHashOut<C::F>,
    ) -> Self {

        let verify_single_end_cap = GUTAVerifySingleEndCapCircuit::<C,D>::new(
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint
        );

        let verify_two_end_cap = GUTAVerifyTwoEndCapCircuit::<C,D>::new(
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint
        );

        let guta_proof_common_data: &CommonCircuitData<C::F, D> = verify_two_end_cap.get_common_circuit_data_ref();
        let guta_proof_verifier_data_cap_height: usize = verify_single_end_cap.get_verifier_config_ref().constants_sigmas_cap.height();
        
        let verify_two_guta = GUTAVerifyTwoGUTACircuit::<C,D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
        );

        let verify_left_guta_right_end_cap = GUTAVerifyLeftGUTARightEndCapCircuit::<C,D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint,
        );

        let verify_left_end_cap_right_guta = GUTAVerifyLeftEndCapRightGUTACircuit::<C,D>::new(
            guta_proof_common_data,
            guta_proof_verifier_data_cap_height,
            end_cap_proof_common_data,
            end_cap_proof_verifier_data_cap_height,
            known_end_cap_fingerprint,
        );

        let verify_guta_register_users = GUTAVerifyGUTARegisterUsersCircuit::<C,D>::new(guta_proof_common_data, guta_proof_verifier_data_cap_height, 16);


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
                ],
            )
            .unwrap();


        let verify_single_end_cap_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_two_end_cap_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_two_guta_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_left_guta_right_end_cap_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_left_end_cap_right_guta_whitelist_proof = guta_circuit_whitelist_proofs.pop().unwrap();
        let verify_guta_register_users_whitelist_proof =guta_circuit_whitelist_proofs.pop().unwrap();
        Self {
            end_cap_fingerprint: known_end_cap_fingerprint,
            verify_single_end_cap,
            verify_two_end_cap,
            verify_two_guta,
            verify_left_guta_right_end_cap,
            verify_left_end_cap_right_guta,
            verify_guta_register_users,

            guta_circuit_whitelist_root: verify_two_guta_whitelist_proof.root,
            verify_single_end_cap_whitelist_proof,
            verify_two_end_cap_whitelist_proof,
            verify_two_guta_whitelist_proof,
            verify_left_guta_right_end_cap_whitelist_proof,
            verify_left_end_cap_right_guta_whitelist_proof,
            verify_guta_register_users_whitelist_proof,
        }
    }

    pub fn print_common_config(&self) {
        println!("\n\n\n\n================================\n[verify_single_end_cap.common]:\n{:?}", self.verify_single_end_cap.get_common_circuit_data_ref());
        println!("================================\n[verify_two_end_cap.common]:\n{:?}", self.verify_two_end_cap.get_common_circuit_data_ref());
        println!("================================\n[verify_two_guta.common]:\n{:?}", self.verify_two_guta.get_common_circuit_data_ref());
        println!("================================\n[verify_left_guta_right_end_cap.common]:\n{:?}", self.verify_left_guta_right_end_cap.get_common_circuit_data_ref());
        println!("================================\n[verify_left_end_cap_right_guta.common]:\n{:?}", self.verify_left_end_cap_right_guta.get_common_circuit_data_ref());
        println!("================================\n[verify_guta_register_users_whitelist_proof.common]:\n{:?}", self.verify_guta_register_users.get_common_circuit_data_ref());
        println!("===============================\n\n\n\n");
    }

    /*
    pub fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>) {
        info_store.register_circuit(
            LocalCircuitType::GUTASingleEndCap.into(),
            self.verify_single_end_cap.get_fingerprint(),
            self.verify_single_end_cap.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::GUTATwoEndCap.into(),
            self.verify_two_end_cap.get_fingerprint(),
            self.verify_two_end_cap.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::GUTATwoGUTA.into(),
            self.verify_two_guta.get_fingerprint(),
            self.verify_two_guta.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::GUTALeftGUTARightEndCap.into(),
            self.verify_left_guta_right_end_cap.get_fingerprint(),
            self.verify_left_guta_right_end_cap.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::GUTALeftEndCapRightGUTA.into(),
            self.verify_left_end_cap_right_guta.get_fingerprint(),
            self.verify_left_end_cap_right_guta.get_verifier_config_ref().into()
        );


        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::GUTASingleEndCap.into(),
            self.verify_single_end_cap_whitelist_proof.clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::GUTATwoEndCap.into(),
            self.verify_two_end_cap_whitelist_proof.clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::GUTATwoGUTA.into(),
            self.verify_two_guta_whitelist_proof.clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::GUTALeftGUTARightEndCap.into(),
            self.verify_left_guta_right_end_cap_whitelist_proof.clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::GUTALeftEndCapRightGUTA.into(),
            self.verify_left_end_cap_right_guta_whitelist_proof.clone(),
        );
        

    }
    */
}



