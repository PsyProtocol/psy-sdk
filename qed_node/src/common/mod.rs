use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_core::data::qhashout::QHashOut;
use psy_core::job::id::{QProvingJobDataID, ProvingJobCircuitType};
use psy_data::config::store_config::QEDProof;
use tracing::info;

pub mod api_request_id;
pub mod jobs;
pub mod traits;
pub mod verifier;

pub fn log_proof_details(prefix: &str, job_id: QProvingJobDataID, proof: &QEDProof) {
    let job_id_hex = hex::encode(job_id.to_fixed_bytes());
    info!("{} - Job ID (hex): {}", prefix, job_id_hex);
    info!("{} - Circuit type: {:?}", prefix, job_id.circuit_type);
    info!("{} - Public inputs length: {}", prefix, proof.public_inputs.len());

    match job_id.circuit_type {
        // 19 public inputs: [commitment(4), worker_public_key(4), pm_jobs_completed(3), circuit_whitelist(4), state_transition_hash(4)]
        ProvingJobCircuitType::AppendUserRegistrationTree |
        ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate |
        ProvingJobCircuitType::BatchDeployContracts => {
            if proof.public_inputs.len() >= 19 {
                let commitment = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                let worker_public_key = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                let pm_jobs_completed = &proof.public_inputs[8..11];
                let circuit_whitelist = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[11..15]);
                let state_transition_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[15..19]);

                info!("{} - [0..4] Commitment: {}", prefix, commitment);
                info!("{} - [4..8] Worker public key: {}", prefix, worker_public_key);
                info!("{} - [8..11] PM jobs completed: [{}, {}, {}]", prefix,
                    pm_jobs_completed[0], pm_jobs_completed[1], pm_jobs_completed[2]);
                info!("{} - [11..15] Circuit whitelist: {}", prefix, circuit_whitelist);
                info!("{} - [15..19] State transition hash: {}", prefix, state_transition_hash);

                match job_id.circuit_type {
                    ProvingJobCircuitType::AppendUserRegistrationTree => {
                        info!("{} - Type: User Registration Leaf Circuit", prefix);
                    }
                    ProvingJobCircuitType::BatchDeployContracts => {
                        info!("{} - Type: Deploy Contracts Leaf Circuit", prefix);
                    }
                    _ => {}
                }
            }
        }

        // 19 public inputs for aggregation circuits: same layout as above but different semantics
        ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => {
            if proof.public_inputs.len() >= 19 {
                let commitment = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                let worker_public_key = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                let pm_jobs_completed = &proof.public_inputs[8..11];
                let allowed_circuit_hashes_root = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[11..15]);
                let state_transition_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[15..19]);

                info!("{} - Type: User Registration Aggregation Circuit", prefix);
                info!("{} - [0..4] Commitment: {}", prefix, commitment);
                info!("{} - [4..8] Worker public key: {}", prefix, worker_public_key);
                info!("{} - [8..11] PM jobs completed: [{}, {}, {}]", prefix,
                    pm_jobs_completed[0], pm_jobs_completed[1], pm_jobs_completed[2]);
                info!("{} - [11..15] Allowed circuit hashes root: {}", prefix, allowed_circuit_hashes_root);
                info!("{} - [15..19] State transition hash: {}", prefix, state_transition_hash);
            }
        }

        // 19 public inputs for aggregation circuits: same layout as standard aggregation
        ProvingJobCircuitType::BatchDeployContractsAggregate => {
            if proof.public_inputs.len() >= 19 {
                let commitment = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                let worker_public_key = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                let pm_jobs_completed = &proof.public_inputs[8..11];
                let allowed_circuit_hashes_root = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[11..15]);
                let state_transition_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[15..19]);

                info!("{} - Type: Deploy Contracts Aggregation Circuit", prefix);
                info!("{} - [0..4] Commitment: {}", prefix, commitment);
                info!("{} - [4..8] Worker public key: {}", prefix, worker_public_key);
                info!("{} - [8..11] PM jobs completed: [{}, {}, {}]", prefix,
                    pm_jobs_completed[0], pm_jobs_completed[1], pm_jobs_completed[2]);
                info!("{} - [11..15] Allowed circuit hashes root: {}", prefix, allowed_circuit_hashes_root);
                info!("{} - [15..19] State transition hash: {}", prefix, state_transition_hash);
            }
        }

        // Special 19 public inputs for AggUserRegisterDeployContractsGUTA: [state_transition_hash(4), user_registration_final(4), deploy_contracts_final(4), guta_final(4), pm_jobs_completed(3)]
        ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA => {
            if proof.public_inputs.len() >= 19 {
                let state_transition_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                let user_registration_final = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                let deploy_contracts_final = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[8..12]);
                let guta_final = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[12..16]);
                let pm_jobs_completed = &proof.public_inputs[16..19];

                info!("{} - Type: Aggregated User Registration + Deploy Contracts + GUTA", prefix);
                info!("{} - [0..4] State transition hash: {}", prefix, state_transition_hash);
                info!("{} - [4..8] User registration final: {}", prefix, user_registration_final);
                info!("{} - [8..12] Deploy contracts final: {}", prefix, deploy_contracts_final);
                info!("{} - [12..16] GUTA final: {}", prefix, guta_final);
                info!("{} - [16..19] PM jobs completed: [{}, {}, {}]", prefix,
                    pm_jobs_completed[0], pm_jobs_completed[1], pm_jobs_completed[2]);
            }
        }

        // 15 public inputs for GUTA circuits: [commitment(4), worker_public_key(4), pm_jobs_completed(3), guta_header_hash(4)]
        ProvingJobCircuitType::GUTATwoGUTA |
        ProvingJobCircuitType::GUTATwoEndCap |
        ProvingJobCircuitType::GUTALeftGUTARightEndCap |
        ProvingJobCircuitType::GUTALeftEndCapRightGUTA |
        ProvingJobCircuitType::GUTAVerifyToCap |
        ProvingJobCircuitType::GUTANoChange |
        ProvingJobCircuitType::GUTASingleEndCap |
        ProvingJobCircuitType::GUTAOnlyRegisterUsers |
        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade |
        ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade |
        ProvingJobCircuitType::GUTARegisterUsers => {
            if proof.public_inputs.len() >= 15 {
                let commitment = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                let worker_public_key = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                let pm_jobs_completed = &proof.public_inputs[8..11];
                let guta_header_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[11..15]);

                info!("{} - Type: GUTA Circuit", prefix);
                info!("{} - [0..4] Commitment: {}", prefix, commitment);
                info!("{} - [4..8] Worker public key: {}", prefix, worker_public_key);
                info!("{} - [8..11] PM jobs completed: [{}, {}, {}]", prefix,
                    pm_jobs_completed[0], pm_jobs_completed[1], pm_jobs_completed[2]);
                info!("{} - [11..15] GUTA header hash: {}", prefix, guta_header_hash);
            }
        }

        _ => {}
    }
}

pub mod slot;
pub mod clock;
pub mod retry;
pub mod whitelist;
pub mod health;
