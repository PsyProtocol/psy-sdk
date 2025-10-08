use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_core::data::qhashout::QHashOut;
use qed_core::job::id::{QProvingJobDataID, ProvingJobCircuitType};
use qed_data::config::store_config::QEDProof;
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
        ProvingJobCircuitType::BatchDeployContracts |
        ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA => {
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
                    ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA => {
                        info!("{} - Type: Aggregated User Registration + Deploy Contracts + GUTA", prefix);
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
            } else if proof.public_inputs.len() >= 8 {
                // Fallback for old/different version with 8 inputs
                let allowed_circuit_hashes_root = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                let state_transition_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                info!("{} - Type: Deploy Contracts Aggregation Circuit (Legacy 8-input version)", prefix);
                info!("{} - [0..4] Allowed circuit hashes root: {}", prefix, allowed_circuit_hashes_root);
                info!("{} - [4..8] State transition hash: {}", prefix, state_transition_hash);
            }
        }

        // 12 public inputs for GUTA circuits: [allowed_circuit_hashes_root(4), state_transition_hash(4), events_hash(4)]
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
            if proof.public_inputs.len() >= 12 {
                let allowed_circuit_hashes_root = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                let state_transition_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                let events_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[8..12]);

                info!("{} - Type: GUTA Circuit", prefix);
                info!("{} - [0..4] Allowed circuit hashes root: {}", prefix, allowed_circuit_hashes_root);
                info!("{} - [4..8] State transition hash: {}", prefix, state_transition_hash);
                info!("{} - [8..12] Events hash: {}", prefix, events_hash);
            }
        }

        _ => {
            // Fallback for unknown circuit types
            if proof.public_inputs.len() >= 4 {
                let first_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[0..4]);
                info!("{} - [0..4] First hash: {}", prefix, first_hash);
            }
            if proof.public_inputs.len() >= 8 {
                let second_hash = QHashOut::<GoldilocksField>::from_felt_slice(&proof.public_inputs[4..8]);
                info!("{} - [4..8] Second hash: {}", prefix, second_hash);
            }
            info!("{} - Type: Unknown circuit type", prefix);
        }
    }
}

pub mod slot;
pub mod clock;
pub mod retry;
pub mod whitelist;
