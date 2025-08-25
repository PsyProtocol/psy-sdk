use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use kvq::traits::KVQSerializable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::HashOut;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::id::{JobProof, ProvingJobCircuitType, QProvingJobDataID};
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
    job_proof: JobProof,
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

    let mut job_infos = if args.jobs.is_empty() {
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

            ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA
            | ProvingJobCircuitType::GenerateRollupStateTransitionProof => {
                info!(
                    "Skipping AggUserRegisterDeployContractsGUTA, GenerateRollupStateTransitionProof job (not supported for rewards)"
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
                let non_zero_siblings = job_proof
                    .siblings
                    .iter()
                    .filter(|s| s.hash.0.elements.iter().any(|e| e.0 != 0))
                    .count();

                info!(
                    "Job {}: type={}, circuit_type={:?}, non_zero_siblings={}, realm={}, job_id={:?}",
                    claims.len(),
                    job_type,
                    job_info.job_id.circuit_type,
                    non_zero_siblings,
                    matches!(job_info.location, JobLocation::Realm(_)),
                    job_info.job_id
                );

                info!("Complete JobProof structure:");
                info!("  Value: [{}, {}, {}, {}]",
                    job_proof.value.0.elements[0].0,
                    job_proof.value.0.elements[1].0,
                    job_proof.value.0.elements[2].0,
                    job_proof.value.0.elements[3].0
                );

                info!("  Siblings ({} total):", job_proof.siblings.len());
                for (i, sibling) in job_proof.siblings.iter().enumerate() {
                    let is_zero = sibling.hash.0.elements.iter().all(|e| e.0 == 0);
                    if !is_zero {
                        info!("    [{}]: hash=[{}, {}, {}, {}], is_left={}",
                            i,
                            sibling.hash.0.elements[0].0,
                            sibling.hash.0.elements[1].0,
                            sibling.hash.0.elements[2].0,
                            sibling.hash.0.elements[3].0,
                            sibling.is_left
                        );
                    }
                }

                info!("  Root: [{}, {}, {}, {}]",
                    job_proof.root.0.elements[0].0,
                    job_proof.root.0.elements[1].0,
                    job_proof.root.0.elements[2].0,
                    job_proof.root.0.elements[3].0
                );

                claims.push(JobClaim {
                    job_id: job_info.job_id.task_index as u64,
                    job_type,
                    job_proof,
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

    for (i, claim) in claims.iter().enumerate() {
        info!(
            "Claim {}: job_id={}, job_type={}, value[0]={}, root[0]={}",
            i,
            claim.job_id,
            claim.job_type,
            claim.job_proof.value.0.elements[0].0,
            claim.job_proof.root.0.elements[0].0
        );
    }

    let mut inputs = vec![args.checkpoint_id];
    let claim_count = claims.len().min(32);

    for i in 0..32 {
        if i < claim_count {
            let claim = &claims[i];

            inputs.push(claim.job_type);

            inputs.push(claim.job_proof.value.0.elements[0].0);
            inputs.push(claim.job_proof.value.0.elements[1].0);
            inputs.push(claim.job_proof.value.0.elements[2].0);
            inputs.push(claim.job_proof.value.0.elements[3].0);

            for j in 0..32 {
                if j < claim.job_proof.siblings.len() {
                    let sibling = &claim.job_proof.siblings[j];
                    inputs.push(sibling.hash.0.elements[0].0);
                    inputs.push(sibling.hash.0.elements[1].0);
                    inputs.push(sibling.hash.0.elements[2].0);
                    inputs.push(sibling.hash.0.elements[3].0);
                    inputs.push(if sibling.is_left { 1 } else { 0 });
                } else {
                    for _ in 0..5 {
                        inputs.push(0);
                    }
                }
            }

            inputs.push(claim.job_proof.root.0.elements[0].0);
            inputs.push(claim.job_proof.root.0.elements[1].0);
            inputs.push(claim.job_proof.root.0.elements[2].0);
            inputs.push(claim.job_proof.root.0.elements[3].0);
        } else {
            inputs.push(0);
            for _ in 0..4 {
                inputs.push(0);
            }
            for _ in 0..160 {
                inputs.push(0);
            }
            for _ in 0..4 {
                inputs.push(0);
            }
        }
    }
    let contract_call_args = vec![ContractCallArgs {
        contract_id: args.contract_id,
        method_name: "batch_claim_pm_rewards".to_string(),
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
    let job_proof = match &job_info.location {
        JobLocation::Realm(realm_id) => {
            let (realm_proof, root_job_id) = provider.get_job_proof_from_realm(
                *realm_id,
                checkpoint_id,
                job_info.job_id.clone(),
            )?;
            info!(
                "DEBUG: Realm {} proof - value[0]={}, root[0]={}, siblings_len={}, root_job_id={:?}",
                realm_id,
                realm_proof.value.0.elements[0].0,
                realm_proof.root.0.elements[0].0,
                realm_proof.siblings.len(),
                root_job_id
            );

            match provider.get_job_proof_from_coordinator(checkpoint_id, root_job_id) {
                Ok((coordinator_proof, _)) => {
                    info!("DEBUG: Coordinator proof for root_job_id - value[0]={}, root[0]={}, siblings_len={}",
                        coordinator_proof.value.0.elements[0].0,
                        coordinator_proof.root.0.elements[0].0,
                        coordinator_proof.siblings.len()
                    );

                    let mut combined_siblings = realm_proof.siblings.clone();

                    if realm_proof.root.0.elements != coordinator_proof.value.0.elements {
                        info!("WARNING: Realm proof root doesn't match coordinator proof value - this may indicate a problem");
                    }

                    combined_siblings.extend(coordinator_proof.siblings);

                    qed_core::job::id::JobProof {
                        value: realm_proof.value,
                        siblings: combined_siblings,
                        root: coordinator_proof.root,
                    }
                }
                Err(e) => {
                    info!(
                        "DEBUG: Failed to get coordinator proof: {}, using realm proof only",
                        e
                    );
                    realm_proof
                }
            }
        }
        JobLocation::Coordinator => {
            let (proof, _) =
                provider.get_job_proof_from_coordinator(checkpoint_id, job_info.job_id.clone())?;
            info!(
                "DEBUG: Coordinator proof - value[0]={}, root[0]={}, siblings_len={}",
                proof.value.0.elements[0].0,
                proof.root.0.elements[0].0,
                proof.siblings.len()
            );
            proof
        }
    };

    Ok(job_proof)
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

    if let Some(coordinator_jobs) = tracker.coordinator.get(&target_checkpoint_id) {
        for job_hex in coordinator_jobs {
            let job_id = parse_job_id_from_hex(job_hex)?;
            job_infos.push(JobInfo {
                job_id,
                location: JobLocation::Coordinator,
            });
        }
    }

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
