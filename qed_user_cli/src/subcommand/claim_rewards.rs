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
use qed_core::job::id::{VariableHeightRewardMerkleProof, ProvingJobCircuitType, QProvingJobDataID};
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

    let mut proofs = Vec::new();

    for job_info in &job_infos {
        if proofs.len() >= 32 {
            info!("Reached maximum of 32 proofs, stopping processing");
            break;
        }

        info!("Processing job: {:?}", job_info.job_id);

        match job_info.job_id.circuit_type {
            ProvingJobCircuitType::GUTAOnlyRegisterUsers
            | ProvingJobCircuitType::GUTARegisterUsers
            | ProvingJobCircuitType::GUTATwoEndCap
            | ProvingJobCircuitType::GUTATwoGUTA
            | ProvingJobCircuitType::GUTALeftEndCapRightGUTA
            | ProvingJobCircuitType::GUTALeftGUTARightEndCap
            | ProvingJobCircuitType::GUTASingleEndCap
            | ProvingJobCircuitType::GUTAVerifyToCap
            | ProvingJobCircuitType::GUTANoChange => {}

            _ => {
                info!(
                    "Skipping non-GUTA job type: {:?}",
                    job_info.job_id.circuit_type
                );
                continue;
            }
        };

        match get_job_proof(&provider, &job_info, args.checkpoint_id) {
            Ok(job_proof) => {
                let non_zero_siblings = job_proof
                    .top_siblings
                    .iter()
                    .filter(|s| s.sibling_branch.0.elements.iter().any(|e| e.0 != 0))
                    .count();

                info!(
                    "Job {}: circuit_type={:?}, non_zero_siblings={}, realm={}, job_id={:?}",
                    proofs.len(),
                    job_info.job_id.circuit_type,
                    non_zero_siblings,
                    matches!(job_info.location, JobLocation::Realm(_)),
                    job_info.job_id
                );

                info!("Complete VariableHeightRewardMerkleProof structure:");
                info!("  Left branch: [{}, {}, {}, {}]",
                    job_proof.left_branch.0.elements[0].0,
                    job_proof.left_branch.0.elements[1].0,
                    job_proof.left_branch.0.elements[2].0,
                    job_proof.left_branch.0.elements[3].0
                );
                info!("  Right branch: [{}, {}, {}, {}]",
                    job_proof.right_branch.0.elements[0].0,
                    job_proof.right_branch.0.elements[1].0,
                    job_proof.right_branch.0.elements[2].0,
                    job_proof.right_branch.0.elements[3].0
                );
                info!("  Reward leaf: [{}, {}, {}, {}]",
                    job_proof.reward_leaf.0.elements[0].0,
                    job_proof.reward_leaf.0.elements[1].0,
                    job_proof.reward_leaf.0.elements[2].0,
                    job_proof.reward_leaf.0.elements[3].0
                );
                info!("  Height: {}, Index: {}",
                    job_proof.proof_height.0,
                    job_proof.index.0
                );

                info!("  Top siblings ({} total):", job_proof.top_siblings.len());
                for (i, sibling) in job_proof.top_siblings.iter().enumerate() {
                    let is_zero = sibling.sibling_branch.0.elements.iter().all(|e| e.0 == 0);
                    if !is_zero {
                        info!("    [{}]: branch=[{}, {}, {}, {}], reward_leaf=[{}, {}, {}, {}]",
                            i,
                            sibling.sibling_branch.0.elements[0].0,
                            sibling.sibling_branch.0.elements[1].0,
                            sibling.sibling_branch.0.elements[2].0,
                            sibling.sibling_branch.0.elements[3].0,
                            sibling.sibling_reward_leaf.0.elements[0].0,
                            sibling.sibling_reward_leaf.0.elements[1].0,
                            sibling.sibling_reward_leaf.0.elements[2].0,
                            sibling.sibling_reward_leaf.0.elements[3].0
                        );
                    }
                }


                proofs.push(job_proof);
            }
            Err(e) => {
                info!("Skipping job due to error: {:?} - {}", job_info.job_id, e);
                continue;
            }
        }
    }

    if proofs.is_empty() {
        return Err(anyhow::format_err!("No valid GUTA proofs found"));
    }

    info!("Processing {} GUTA proofs for checkpoint {}", proofs.len(), args.checkpoint_id);

    let mut contract_call_args = Vec::new();
    for (i, proof) in proofs.iter().enumerate() {
        info!("Preparing GUTA reward {}/{}: height={}, index={}", 
              i + 1, proofs.len(), proof.proof_height.0, proof.index.0);

        let inputs = vec![
            args.checkpoint_id,
            proof.proof_height.0,
            proof.index.0,
            proof.left_branch.0.elements[0].0,
            proof.left_branch.0.elements[1].0,
            proof.left_branch.0.elements[2].0,
            proof.left_branch.0.elements[3].0,
            proof.right_branch.0.elements[0].0,
            proof.right_branch.0.elements[1].0,
            proof.right_branch.0.elements[2].0,
            proof.right_branch.0.elements[3].0,
            proof.reward_leaf.0.elements[0].0,
            proof.reward_leaf.0.elements[1].0,
            proof.reward_leaf.0.elements[2].0,
            proof.reward_leaf.0.elements[3].0,
        ];

        let mut all_inputs = inputs;
        for sibling in &proof.top_siblings {
            all_inputs.extend(vec![
                sibling.sibling_branch.0.elements[0].0,
                sibling.sibling_branch.0.elements[1].0,
                sibling.sibling_branch.0.elements[2].0,
                sibling.sibling_branch.0.elements[3].0,
                sibling.sibling_reward_leaf.0.elements[0].0,
                sibling.sibling_reward_leaf.0.elements[1].0,
                sibling.sibling_reward_leaf.0.elements[2].0,
                sibling.sibling_reward_leaf.0.elements[3].0,
            ]);
        }

        contract_call_args.push(ContractCallArgs {
            contract_id: args.contract_id,
            method_name: "advance_and_claim_guta_proof".to_string(),
            inputs: all_inputs,
        });
    }
    info!("Preparing checkpoint seal for {}", args.checkpoint_id);
    contract_call_args.push(ContractCallArgs {
        contract_id: args.contract_id,
        method_name: "advance_to_checkpoint_and_seal".to_string(),
        inputs: vec![args.checkpoint_id],
    });
    info!("Executing {} contract calls in single UPS transaction", contract_call_args.len());
    wallet_session.exec_contract_call_with_sign_type(
        user_pk_hash,
        contract_call_args,
        args.sign_type.clone(),
        fingerprint,
        Some(args.contract_id),
        vec![],
    )?;

    info!("Successfully processed {} GUTA rewards and sealed checkpoint {}", proofs.len(), args.checkpoint_id);

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
) -> Result<qed_core::job::id::VariableHeightRewardMerkleProof> {
    let job_proof = match &job_info.location {
        JobLocation::Realm(realm_id) => {
            let (realm_proof, root_job_id) = provider.get_job_proof_from_realm(
                *realm_id,
                checkpoint_id,
                job_info.job_id.get_output_id(),
            )?;
            info!(
                "DEBUG: Realm {} proof - left_branch[0]={}, height={}, top_siblings_len={}, root_job_id={:?}",
                realm_id,
                realm_proof.left_branch.0.elements[0].0,
                realm_proof.proof_height.0,
                realm_proof.top_siblings.len(),
                root_job_id
            );

            match provider.get_job_proof_from_coordinator(checkpoint_id, root_job_id.get_output_id()) {
                Ok((coordinator_proof, _)) => {
                    info!("DEBUG: Coordinator proof for root_job_id - left_branch[0]={}, height={}, top_siblings_len={}",
                        coordinator_proof.left_branch.0.elements[0].0,
                        coordinator_proof.proof_height.0,
                        coordinator_proof.top_siblings.len()
                    );

                    realm_proof.combine_with(coordinator_proof)
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
                provider.get_job_proof_from_coordinator(checkpoint_id, job_info.job_id.get_output_id())?;
            info!(
                "DEBUG: Coordinator proof - left_branch[0]={}, height={}, top_siblings_len={}",
                proof.left_branch.0.elements[0].0,
                proof.proof_height.0,
                proof.top_siblings.len()
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
