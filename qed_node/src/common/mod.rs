use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_core::data::qhashout::QHashOut;
use qed_core::job::id::{QProvingJobDataID, ProvingJobCircuitType};
use tracing::info;

pub mod api_request_id;
pub mod jobs;
pub mod traits;
pub mod verifier;
const D: usize = 2;
pub type ConcreteProofWithPublicInputs =
    ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, D>;

pub fn log_proof_details(prefix: &str, job_id: QProvingJobDataID, proof: &ConcreteProofWithPublicInputs) {
    let job_id_hex = hex::encode(job_id.to_fixed_bytes());
    info!("{} - Job ID (hex): {}", prefix, job_id_hex);
    info!("{} - Circuit type: {:?}", prefix, job_id.circuit_type);
    
    if proof.public_inputs.len() >= 12 {
        let commitment = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
        let worker_public_key = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
        let data_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[8..12]);
        
        info!("{} - Commitment: {}", prefix, commitment.to_string());
        info!("{} - Worker public key: {}", prefix, worker_public_key.to_string());
        info!("{} - Data hash: {}", prefix, data_hash.to_string());
        
        match job_id.circuit_type {
            ProvingJobCircuitType::AppendUserRegistrationTree |
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => {
                info!("{} - Register users job - data_hash is user_registration_hash", prefix);
            }
            ProvingJobCircuitType::BatchDeployContracts |
            ProvingJobCircuitType::BatchDeployContractsAggregate => {
                info!("{} - Deploy contracts job - data_hash is contracts_hash", prefix);
            }
            ProvingJobCircuitType::GUTATwoGUTA |
            ProvingJobCircuitType::GUTATwoEndCap |
            ProvingJobCircuitType::GUTALeftGUTARightEndCap |
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA |
            ProvingJobCircuitType::GUTAVerifyToCap |
            ProvingJobCircuitType::GUTANoChange |
            ProvingJobCircuitType::GUTASingleEndCap |
            ProvingJobCircuitType::GUTAOnlyRegisterUsers |
            ProvingJobCircuitType::GUTARegisterUsers => {
                info!("{} - GUTA job - data_hash is guta_hash", prefix);
            }
            ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA => {
                if proof.public_inputs.len() >= 16 {
                    let state_transition_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                    let register_users_root = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                    let deploy_contracts_root = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[8..12]);
                    let gutas_root = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[12..16]);
                    
                    info!("{} - State Part 1 (AggUserRegisterDeployContractsGUTA):", prefix);
                    info!("{}   State transition hash: {}", prefix, state_transition_hash.to_string());
                    info!("{}   Register users root: {}", prefix, register_users_root.to_string());
                    info!("{}   Deploy contracts root: {}", prefix, deploy_contracts_root.to_string());
                    info!("{}   GUTAs root: {}", prefix, gutas_root.to_string());
                    info!("{}   PM Rewards Commitment = {{register_users_root, deploy_contracts_root, gutas_root}}", prefix);
                }
            }
            _ => {}
        }
    }
}

pub mod slot;
pub mod clock;
