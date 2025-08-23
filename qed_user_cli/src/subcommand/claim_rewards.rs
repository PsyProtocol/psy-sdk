use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use kvq::traits::KVQSerializable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::id::{ProvingJobCircuitType, QProvingJobDataID};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_data::config::store_config::QEDHasher;
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_prover::local::{
    args::{ContractCallArgs, SignType},
    provider::{RpcConfig, RpcProvider},
};
use qed_prover::session::WalletSession;
use serde_json::json;
use tracing::info;

use super::args::ClaimRewardsArgs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RealmJobData {
    id: u32,
    checkpoints: HashMap<u64, Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WorkerJobTracker {
    coordinator: HashMap<u64, Vec<String>>,
    realms: Vec<RealmJobData>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum JobLocation {
    Realm(u64),
    Coordinator,
}

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
struct JobInfo {
    job_id: QProvingJobDataID,
    location: JobLocation,
}

#[derive(Debug, Clone)]
struct JobClaim {
    job_id: u64,
    job_type: u64,
    job_proof: qed_core::job::id::JobProof,
    reward: u64,
}

pub fn run(args: ClaimRewardsArgs) -> Result<()> {
    info!(
        "Starting claim rewards with checkpoint_id: {}",
        args.checkpoint_id
    );

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
    let private_key = QHashOut::from(Hash256::from_hex_string(&args.private_key)?);

    let provider = RpcProvider::new_with_config(&rpc_config)?;
    let mut wallet_session = WalletSession::new(&rpc_config)?;
    let fingerprint = if args.fingerprint.is_some() {
        Some(
            QHashOut::<F>::from_str(&args.fingerprint.as_ref().unwrap())
                .map_err(|e| anyhow::format_err!("Failed to parse fingerprint: {}", e))?,
        )
    } else {
        None
    };

    let user_pk_hash =
        wallet_session.add_user_with_type(private_key, args.sign_type.clone(), fingerprint)?;

    let job_infos = if args.jobs.is_empty() {
        load_jobs_from_tracker_file(&user_pk_hash, args.checkpoint_id)?
    } else {
        parse_job_specs(&args.jobs)?
    };
    let mut claims = Vec::new();

    for job_info in &job_infos {
        if claims.len() >= 32 {
            info!("Reached maximum of 32 claims, stopping processing");
            break;
        }

        info!("Processing job: {:?}", job_info.job_id);

        let job_type = match job_info.job_id.circuit_type {
            ProvingJobCircuitType::AppendUserRegistrationTree
            | ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
            | ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => 0,

            ProvingJobCircuitType::GUTAOnlyRegisterUsers
            | ProvingJobCircuitType::GUTARegisterUsers
            | ProvingJobCircuitType::GUTATwoEndCap
            | ProvingJobCircuitType::GUTATwoGUTA
            | ProvingJobCircuitType::GUTALeftEndCapRightGUTA
            | ProvingJobCircuitType::GUTALeftGUTARightEndCap
            | ProvingJobCircuitType::GUTASingleEndCap
            | ProvingJobCircuitType::GUTAVerifyToCap
            | ProvingJobCircuitType::GUTANoChange => 1,

            ProvingJobCircuitType::BatchDeployContracts
            | ProvingJobCircuitType::BatchDeployContractsAggregate
            | ProvingJobCircuitType::DummyBatchDeployContractsAggregate => 2,

            ProvingJobCircuitType::GenerateRollupStateTransitionProof => {
                info!(
                    "Skipping GenerateRollupStateTransitionProof job (not supported for rewards)"
                );
                continue;
            }

            _ => {
                info!(
                    "Skipping unsupported job type: {:?}",
                    job_info.job_id.circuit_type
                );
                continue;
            }
        };

        match get_job_proof(&provider, &job_info, args.checkpoint_id) {
            Ok(job_proof) => {
                info!(
                    "Job type: {}, Proof value: {:?}, Proof root: {:?}",
                    job_type, job_proof.value, job_proof.root
                );

                let non_zero_siblings = job_proof
                    .siblings
                    .iter()
                    .filter(|s| s.hash.0.elements.iter().any(|e| e.0 != 0))
                    .count();
                info!("Non-zero siblings: {}", non_zero_siblings);

                let reward = calculate_reward(&job_info.job_id);

                claims.push(JobClaim {
                    job_id: job_info.job_id.task_index as u64,
                    job_type,
                    job_proof,
                    reward,
                });
            }
            Err(e) => {
                info!("Skipping job due to error: {:?} - {}", job_info.job_id, e);
                continue;
            }
        }
    }

    if claims.is_empty() {
        return Err(anyhow::format_err!("No valid claims found"));
    }

    info!("Submitting {} claims to rewards contract", claims.len());
    let mut inputs = vec![args.checkpoint_id];
    let claim_count = claims.len().min(32);

    // Format claims array - 32 JobClaim structs
    for i in 0..32 {
        if i < claim_count {
            let claim = &claims[i];

            // JobClaim.job_type
            inputs.push(claim.job_type);

            // JobClaim.proof.value (4 Felts)
            inputs.push(claim.job_proof.value.0.elements[0].0);
            inputs.push(claim.job_proof.value.0.elements[1].0);
            inputs.push(claim.job_proof.value.0.elements[2].0);
            inputs.push(claim.job_proof.value.0.elements[3].0);

            // JobClaim.proof.siblings (32 JobProofSibling structs)
            for j in 0..32 {
                let sibling = &claim.job_proof.siblings[j];
                // sibling_hash (4 Felts)
                inputs.push(sibling.hash.0.elements[0].0);
                inputs.push(sibling.hash.0.elements[1].0);
                inputs.push(sibling.hash.0.elements[2].0);
                inputs.push(sibling.hash.0.elements[3].0);
                // is_left (1 Felt) - convert bool to Felt
                inputs.push(if sibling.is_left { 1 } else { 0 });
            }

            // JobClaim.proof.root (4 Felts)
            inputs.push(claim.job_proof.root.0.elements[0].0);
            inputs.push(claim.job_proof.root.0.elements[1].0);
            inputs.push(claim.job_proof.root.0.elements[2].0);
            inputs.push(claim.job_proof.root.0.elements[3].0);

            // JobClaim.reward
            inputs.push(claim.reward);
        } else {
            // Empty JobClaim padding
            inputs.push(0); // job_type
            for _ in 0..4 {
                inputs.push(0);
            } // value
            for _ in 0..160 {
                inputs.push(0);
            } // siblings (32 * 5)
            for _ in 0..4 {
                inputs.push(0);
            } // root
            inputs.push(0); // reward
        }
    }
    let contract_call_args = vec![ContractCallArgs {
        contract_id: args.contract_id,
        method_name: "claim_batch_job_rewards".to_string(),
        inputs,
    }];
    wallet_session.exec_contract_call_with_sign_type(
        user_pk_hash,
        contract_call_args,
        args.sign_type.clone(),
        fingerprint,
        Some(args.contract_id),
        vec![],
    )?;

    info!(
        "Successfully submitted rewards claim for {} jobs",
        claim_count
    );

    Ok(())
}

fn parse_job_specs(specs: &[String]) -> Result<Vec<JobInfo>> {
    let mut job_infos = Vec::new();

    for spec in specs {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() < 2 {
            return Err(anyhow::format_err!("Invalid job spec format: {}", spec));
        }
        let job_id_bytes = hex::decode(parts[0])
            .map_err(|_| anyhow::format_err!("Invalid job ID hex: {}", parts[0]))?;
        let job_id = QProvingJobDataID::try_from_byte_vec(&job_id_bytes)
            .map_err(|e| anyhow::format_err!("Invalid job ID: {}", e))?;
        let location = if parts[1] == "coordinator" {
            JobLocation::Coordinator
        } else if parts[1] == "realm" && parts.len() > 2 {
            let realm_id = parts[2]
                .parse::<u64>()
                .map_err(|_| anyhow::format_err!("Invalid realm ID: {}", parts[2]))?;
            JobLocation::Realm(realm_id)
        } else {
            return Err(anyhow::format_err!("Invalid location spec: {}", spec));
        };

        job_infos.push(JobInfo { job_id, location });
    }

    Ok(job_infos)
}

fn get_job_proof(
    provider: &RpcProvider,
    job_info: &JobInfo,
    checkpoint_id: u64,
) -> Result<qed_core::job::id::JobProof> {
    // Call the RPC to get the JobProof
    let job_proof = match &job_info.location {
        JobLocation::Realm(realm_id) => {
            provider.get_job_proof_from_realm(*realm_id, checkpoint_id, job_info.job_id.clone())?
        }
        JobLocation::Coordinator => {
            provider.get_job_proof_from_coordinator(checkpoint_id, job_info.job_id.clone())?
        }
    };

    Ok(job_proof)
}

fn calculate_reward(_job_id: &QProvingJobDataID) -> u64 {
    100
}

fn load_jobs_from_tracker_file(
    public_key: &QHashOut<F>,
    target_checkpoint_id: u64,
) -> Result<Vec<JobInfo>> {
    let filename = format!("{}.json", public_key.to_string());

    if !std::path::Path::new(&filename).exists() {
        info!("No job tracker file found: {}", filename);
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&filename)?;
    let tracker: WorkerJobTracker = serde_json::from_str(&content)?;

    let mut job_infos = Vec::new();

    // Load coordinator jobs for the target checkpoint
    if let Some(coordinator_jobs) = tracker.coordinator.get(&target_checkpoint_id) {
        for job_hex in coordinator_jobs {
            let job_id = parse_job_id_from_hex(job_hex)?;
            job_infos.push(JobInfo {
                job_id,
                location: JobLocation::Coordinator,
            });
        }
    }

    // Load realm jobs for the target checkpoint
    for realm in &tracker.realms {
        if let Some(realm_jobs) = realm.checkpoints.get(&target_checkpoint_id) {
            for job_hex in realm_jobs {
                let job_id = parse_job_id_from_hex(job_hex)?;
                job_infos.push(JobInfo {
                    job_id,
                    location: JobLocation::Realm(realm.id as u64),
                });
            }
        }
    }

    info!(
        "Loaded {} jobs from tracker file {} for checkpoint {}",
        job_infos.len(),
        filename,
        target_checkpoint_id
    );
    Ok(job_infos)
}

fn parse_job_id_from_hex(hex_str: &str) -> Result<QProvingJobDataID> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(hex_str)?;
    if bytes.len() != 24 {
        anyhow::bail!(
            "Invalid job ID length: expected 24 bytes, got {}",
            bytes.len()
        );
    }
    QProvingJobDataID::try_from_byte_vec(&bytes)
}
