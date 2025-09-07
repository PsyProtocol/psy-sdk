use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use kvq::traits::KVQSerializable;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::hash_types::HashOut,
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::{
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
    job::id::{ProvingJobCircuitType, QProvingJobDataID, VariableHeightRewardMerkleProof, GUTA_REWARDS_TREE_MAX_HEIGHT},
};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_data::{config::store_config::QEDHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync};
use qed_node::worker::job_tracker::{JobInfo, JobLocation, WorkerJobTracker};
use qed_prover::{
    local::{
        args::{ContractCallArgs, SignType},
        provider::{RpcConfig, RpcProvider},
    },
    session::WalletSession,
};
use serde_json::json;
use tracing::{info, warn};

use super::args::ClaimRewardsArgs;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub fn run(args: ClaimRewardsArgs) -> Result<()> {
    info!("Starting claim rewards with checkpoint_id: {}", args.checkpoint_id);

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
    let private_key = QHashOut::from(Hash256::from_hex_string(&args.private_key)?);

    let provider = RpcProvider::new_with_config(&rpc_config)?;
    let mut wallet_session = WalletSession::new(&rpc_config)?;
    let fingerprint = if args.fingerprint.is_some() {
        Some(QHashOut::<F>::from_str(&args.fingerprint.as_ref().unwrap()).map_err(|e| anyhow::format_err!("Failed to parse fingerprint: {}", e))?)
    } else {
        None
    };

    let user_pk_hash = wallet_session.add_user_with_type(private_key, args.sign_type.clone(), fingerprint)?;

    let mut job_infos = if args.jobs.is_empty() {
        load_jobs_from_tracker_file(&user_pk_hash, args.checkpoint_id)?
    } else {
        parse_job_specs(&args.jobs)?
    };

    let mut all_proofs = Vec::new();

    for job_info in &job_infos {
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
                info!("Skipping non-GUTA job type: {:?}", job_info.job_id.circuit_type);
                continue;
            }
        };

        if let Ok(job_proof) = get_job_proof(&provider, &job_info, args.checkpoint_id) {
            info!("Found GUTA proof for job {}", job_info.job_id.to_hex_string());
            all_proofs.push(job_proof);
        } else {
            warn!("Skipping job {}: failed to get proof", job_info.job_id.to_hex_string());
        }
    }

    if all_proofs.is_empty() {
        return Err(anyhow::format_err!("No valid GUTA proofs found"));
    }

    info!("Found {} GUTA proofs total for checkpoint {}", all_proofs.len(), args.checkpoint_id);

    let checkpoint_leaf = provider.get_checkpoint_leaf_data(args.checkpoint_id)?;
    let fees_collected = checkpoint_leaf.stats.fees_collected.to_canonical_u64();
    let gutas_completed = checkpoint_leaf.stats.pm_jobs_completed.gutas_completed.to_canonical_u64();

    let proposed_reward = if gutas_completed > 0 { fees_collected / gutas_completed } else { 0u64 };

    info!(
        "Checkpoint {} stats: fees_collected={}, gutas_completed={}, fee_per_proof={}",
        args.checkpoint_id, fees_collected, gutas_completed, proposed_reward
    );

    let mining_rewards_contract_id = 2;

    let mut contract_call_args = Vec::new();

    contract_call_args.push(ContractCallArgs {
        contract_id: mining_rewards_contract_id,
        method_name: "start_session".to_string(),
        inputs: vec![args.checkpoint_id],
    });

    for (chunk_index, chunk) in all_proofs.chunks(8).enumerate() {
        info!("Processing chunk {} with {} proofs", chunk_index, chunk.len());

        let mut proof_inputs = Vec::new();
        for i in 0..8 {
            if i < chunk.len() {
                let proof = &chunk[i];
                for j in 0..32 {
                    if j < proof.top_siblings.len() {
                        let sibling = &proof.top_siblings[j];
                        proof_inputs.extend(vec![
                            sibling.sibling_branch.0.elements[0].0,
                            sibling.sibling_branch.0.elements[1].0,
                            sibling.sibling_branch.0.elements[2].0,
                            sibling.sibling_branch.0.elements[3].0,
                            sibling.sibling_reward_leaf.0.elements[0].0,
                            sibling.sibling_reward_leaf.0.elements[1].0,
                            sibling.sibling_reward_leaf.0.elements[2].0,
                            sibling.sibling_reward_leaf.0.elements[3].0,
                        ]);
                    } else {
                        proof_inputs.extend(vec![0u64; 8]);
                    }
                }
                proof_inputs.extend(vec![
                    proof.sibling_branch.0.elements[0].0,
                    proof.sibling_branch.0.elements[1].0,
                    proof.sibling_branch.0.elements[2].0,
                    proof.sibling_branch.0.elements[3].0,
                ]);
                proof_inputs.extend(vec![
                    proof.reward_leaf.0.elements[0].0,
                    proof.reward_leaf.0.elements[1].0,
                    proof.reward_leaf.0.elements[2].0,
                    proof.reward_leaf.0.elements[3].0,
                ]);
                proof_inputs.extend(vec![proof.proof_height.0, proof.index.0]);
            } else {
                proof_inputs.extend(vec![0u64; 266]);
            }
        }

        let mut batch_inputs = vec![args.checkpoint_id];
        batch_inputs.extend(proof_inputs);
        batch_inputs.push(proposed_reward);

        contract_call_args.push(ContractCallArgs {
            contract_id: mining_rewards_contract_id,
            method_name: "batch_claim_guta_rewards".to_string(),
            inputs: batch_inputs,
        });
    }

    contract_call_args.push(ContractCallArgs {
        contract_id: mining_rewards_contract_id,
        method_name: "end_session".to_string(),
        inputs: vec![args.checkpoint_id],
    });

    let token_contract_id = 0;
    contract_call_args.push(ContractCallArgs {
        contract_id: token_contract_id,
        method_name: "simple_claim_pow_rewards".to_string(),
        inputs: vec![args.checkpoint_id],
    });

    info!("Executing {} contract calls in single UPS transaction", contract_call_args.len());
    wallet_session.exec_contract_call_with_sign_type(
        user_pk_hash,
        contract_call_args,
        args.sign_type.clone(),
        fingerprint,
        Some(mining_rewards_contract_id),
        vec![],
    )?;

    info!(
        "Successfully processed {} GUTA rewards for checkpoint {}",
        all_proofs.len(),
        args.checkpoint_id
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
        let job_id_bytes = hex::decode(parts[0]).map_err(|_| anyhow::format_err!("Invalid job ID hex: {}", parts[0]))?;
        let job_id = QProvingJobDataID::try_from_byte_vec(&job_id_bytes).map_err(|e| anyhow::format_err!("Invalid job ID: {}", e))?;
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
) -> anyhow::Result<qed_core::job::id::VariableHeightRewardMerkleProof> {
    let job_proof = match &job_info.location {
        JobLocation::Realm(realm_id) => {
            let (proof, root_job_id) = provider.get_job_proof_from_realm(*realm_id, checkpoint_id, job_info.job_id.get_output_id())?;

            if root_job_id.goal_id != checkpoint_id {
                return Err(anyhow::format_err!(
                    "checkpoint mismatch: job {} was processed in checkpoint {} but expected {}",
                    job_info.job_id.to_hex_string(),
                    root_job_id.goal_id,
                    checkpoint_id
                ));
            }

            proof
        }
        JobLocation::Coordinator => {
            let (proof, root_job_id) = provider.get_job_proof_from_coordinator(checkpoint_id, job_info.job_id.get_output_id())?;

            if root_job_id.goal_id != checkpoint_id {
                return Err(anyhow::format_err!(
                    "checkpoint mismatch: job {} was processed in checkpoint {} but expected {}",
                    job_info.job_id.to_hex_string(),
                    root_job_id.goal_id,
                    checkpoint_id
                ));
            }

            proof
        }
    };

    Ok(job_proof.pad_to_height(GUTA_REWARDS_TREE_MAX_HEIGHT * 2))
}

fn load_jobs_from_tracker_file(public_key: &QHashOut<F>, target_checkpoint_id: u64) -> Result<Vec<JobInfo>> {
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
        anyhow::bail!("Invalid job ID length: expected 24 bytes, got {}", bytes.len());
    }
    QProvingJobDataID::try_from_byte_vec(&bytes)
}
