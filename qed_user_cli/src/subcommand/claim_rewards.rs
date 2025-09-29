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
    config::network_constants::{TOKEN_CONTRACT_ID, MAX_CONTRACT_STATE_TREE_HEIGHT},
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
    job::id::{ProvingJobCircuitType, QProvingJobDataID, VariableHeightRewardMerkleProof, GUTA_REWARDS_TREE_MAX_HEIGHT},
};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_data::{
    config::store_config::QEDHasher,
    traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync},
};
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

#[derive(Clone)]
struct ProofWithCheckpoint {
    checkpoint_id: u64,
    proof: VariableHeightRewardMerkleProof,
    proposed_reward: u64,
}

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

const MINING_REWARDS_CONTRACT_ID: u64 = 1;
const LAST_CLAIMED_CHECKPOINT_SLOT: u64 = 0;

pub fn run(args: ClaimRewardsArgs) -> Result<()> {
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
    let user_id = provider.get_user_id(user_pk_hash)?;

    let latest_l2_block_state = provider.get_latest_l2_block_state()?;
    let latest_checkpoint_id = latest_l2_block_state.checkpoint_id;

    info!("Latest checkpoint: {}", latest_checkpoint_id);

    let last_claimed = match get_last_claimed_checkpoint_id(&provider, user_id, latest_checkpoint_id) {
        Ok(checkpoint) => {
            info!("Last claimed checkpoint: {}", checkpoint);
            checkpoint
        }
        Err(e) => {
            warn!("Failed to query last claimed checkpoint ({}), starting from checkpoint 1", e);
            0
        }
    };

    let start_checkpoint = last_claimed + 1;

    let claim_rewards_cooldown = 0;
    let max_claimable_checkpoint = if latest_checkpoint_id > claim_rewards_cooldown {
        latest_checkpoint_id - claim_rewards_cooldown
    } else {
        0
    };

    if start_checkpoint > max_claimable_checkpoint {
        info!("No new checkpoints to claim (latest: {}, cooldown: {}, max claimable: {})",
              latest_checkpoint_id, claim_rewards_cooldown, max_claimable_checkpoint);
        return Ok(());
    }

    info!("Claiming rewards from checkpoint {} to {} (latest: {}, cooldown: {})",
          start_checkpoint, max_claimable_checkpoint, latest_checkpoint_id, claim_rewards_cooldown);

    let mut checkpoint_jobs: HashMap<u64, Vec<(JobInfo, VariableHeightRewardMerkleProof)>> = HashMap::new();

    for checkpoint_id in start_checkpoint..=max_claimable_checkpoint {
        let mut job_infos = if !args.jobs.is_empty() {
            parse_job_specs(&args.jobs)?
        } else {
            load_jobs_from_tracker_file(&user_pk_hash, checkpoint_id)?
        };

        if job_infos.is_empty() {
            continue;
        }

        for job_info in job_infos {
            match job_info.job_id.circuit_type {
                ProvingJobCircuitType::GUTAOnlyRegisterUsers
                | ProvingJobCircuitType::GUTARegisterUsers
                | ProvingJobCircuitType::GUTATwoEndCap
                | ProvingJobCircuitType::GUTATwoGUTA
                | ProvingJobCircuitType::GUTALeftEndCapRightGUTA
                | ProvingJobCircuitType::GUTALeftGUTARightEndCap
                | ProvingJobCircuitType::GUTASingleEndCap
                | ProvingJobCircuitType::GUTAVerifyToCap
                | ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade
                | ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade
                | ProvingJobCircuitType::GUTANoChange => {}

                _ => {
                    continue;
                }
            };

            if let Ok((actual_checkpoint_id, job_proof)) = get_job_proof(&provider, &job_info, checkpoint_id) {
                checkpoint_jobs.entry(actual_checkpoint_id)
                    .or_insert_with(Vec::new)
                    .push((job_info, job_proof));
            }
        }
    }

    if checkpoint_jobs.is_empty() {
        info!("No valid checkpoints with rewards to claim");
        return Ok(());
    }

    let mut sorted_checkpoints: Vec<_> = checkpoint_jobs.keys().copied().collect();
    sorted_checkpoints.sort();

    let mut all_proofs_with_checkpoints = Vec::new();

    for &checkpoint_id in &sorted_checkpoints {
        let jobs = checkpoint_jobs.get(&checkpoint_id).unwrap();

        let checkpoint_leaf = provider.get_checkpoint_leaf_data(checkpoint_id)?;
        let fees_collected = checkpoint_leaf.stats.fees_collected.to_canonical_u64();
        let gutas_completed = checkpoint_leaf.stats.pm_jobs_completed.gutas_completed.to_canonical_u64();

        let proposed_reward = if gutas_completed > 0 { fees_collected / gutas_completed } else { 0u64 };

        if proposed_reward == 0 {
            warn!("Skipping checkpoint {} due to zero reward (fees_collected={}, gutas_completed={})",
                  checkpoint_id, fees_collected, gutas_completed);
            continue;
        }

        info!("Checkpoint {} - Reward: {}, Jobs: {}", checkpoint_id, proposed_reward, jobs.len());
        for (job_info, _) in jobs {
            info!("  - {} ({})", job_info.job_id.to_hex_string(), match &job_info.location {
                JobLocation::Coordinator => "coordinator".to_string(),
                JobLocation::Realm(id) => format!("realm:{}", id),
            });
        }

        for (_, proof) in jobs {
            all_proofs_with_checkpoints.push(ProofWithCheckpoint {
                checkpoint_id,
                proof: proof.clone(),
                proposed_reward,
            });
        }
    }

    if all_proofs_with_checkpoints.is_empty() {
        info!("No checkpoints with valid rewards to claim");
        return Ok(());
    }

    let mut all_contract_calls = build_claim_calls_for_multi_checkpoints(&all_proofs_with_checkpoints);

    if all_contract_calls.is_empty() {
        info!("No checkpoints with valid rewards to claim");
        return Ok(());
    }

    let last_checkpoint = all_proofs_with_checkpoints
        .last()
        .unwrap()
        .checkpoint_id;

    all_contract_calls.push(ContractCallArgs {
        contract_id: MINING_REWARDS_CONTRACT_ID,
        method_name: "end_session".to_string(),
        inputs: vec![last_checkpoint],
    });

    all_contract_calls.push(ContractCallArgs {
        contract_id: TOKEN_CONTRACT_ID as u64,
        method_name: "simple_claim_pow_rewards".to_string(),
        inputs: vec![last_checkpoint],
    });

    if all_contract_calls.is_empty() {
        info!("No rewards to claim");
        return Ok(());
    }

    info!("Executing {} contract calls in single transaction", all_contract_calls.len());
    wallet_session.exec_contract_call_with_sign_type(
        user_pk_hash,
        all_contract_calls,
        args.sign_type.clone(),
        fingerprint,
        Some(MINING_REWARDS_CONTRACT_ID),
        vec![],
    )?;

    info!("Successfully claimed rewards");

    Ok(())
}

fn get_last_claimed_checkpoint_id(provider: &RpcProvider, user_id: u64, latest_checkpoint_id: u64) -> Result<u64> {
    let proof = provider.get_user_contract_state_tree_merkle_proof(
        latest_checkpoint_id,
        user_id,
        TOKEN_CONTRACT_ID,
        MAX_CONTRACT_STATE_TREE_HEIGHT,
        LAST_CLAIMED_CHECKPOINT_SLOT,
    )?;

    Ok(proof.value.0.elements[1].0)
}

fn build_claim_calls_for_multi_checkpoints(
    all_proofs: &[ProofWithCheckpoint],
) -> Vec<ContractCallArgs> {
    let mut contract_call_args = Vec::new();

    let total_proofs = all_proofs.len();
    let mut proof_index = 0;

    let count_10s = total_proofs / 10;
    let mut remaining = total_proofs % 10;

    let count_5s = remaining / 5;
    remaining = remaining % 5;

    for _ in 0..count_10s {
        let chunk = &all_proofs[proof_index..proof_index + 10];
        let mut batch_inputs = Vec::new();

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.checkpoint_id);
        }

        for proof_with_checkpoint in chunk {
            serialize_proof_to_inputs(&proof_with_checkpoint.proof, &mut batch_inputs);
        }

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.proposed_reward);
        }

        contract_call_args.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "claim_guta_rewards_10".to_string(),
            inputs: batch_inputs,
        });

        proof_index += 10;
    }

    for _ in 0..count_5s {
        let chunk = &all_proofs[proof_index..proof_index + 5];
        let mut batch_inputs = Vec::new();

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.checkpoint_id);
        }

        for proof_with_checkpoint in chunk {
            serialize_proof_to_inputs(&proof_with_checkpoint.proof, &mut batch_inputs);
        }

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.proposed_reward);
        }

        contract_call_args.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "claim_guta_rewards_5".to_string(),
            inputs: batch_inputs,
        });

        proof_index += 5;
    }

    if remaining >= 2 {
        let chunk = &all_proofs[proof_index..proof_index + 2];
        let mut batch_inputs = Vec::new();

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.checkpoint_id);
        }

        for proof_with_checkpoint in chunk {
            serialize_proof_to_inputs(&proof_with_checkpoint.proof, &mut batch_inputs);
        }

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.proposed_reward);
        }

        contract_call_args.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "claim_guta_rewards_2".to_string(),
            inputs: batch_inputs,
        });

        proof_index += 2;
    }

    if proof_index < total_proofs {
        let proof_with_checkpoint = &all_proofs[proof_index];
        let mut proof_inputs = Vec::new();

        serialize_proof_to_inputs(&proof_with_checkpoint.proof, &mut proof_inputs);

        let mut batch_inputs = vec![proof_with_checkpoint.checkpoint_id];
        batch_inputs.extend(proof_inputs);
        batch_inputs.push(proof_with_checkpoint.proposed_reward);

        contract_call_args.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "claim_guta_rewards_1".to_string(),
            inputs: batch_inputs,
        });
    }

    contract_call_args
}

fn serialize_proof_to_inputs(proof: &VariableHeightRewardMerkleProof, inputs: &mut Vec<u64>) {
    for j in 0..GUTA_REWARDS_TREE_MAX_HEIGHT {
        if j < proof.top_siblings.len() {
            let sibling = &proof.top_siblings[j];
            inputs.extend(vec![
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
            inputs.extend(vec![0u64; 8]);
        }
    }
    inputs.extend(vec![
        proof.sibling_branch.0.elements[0].0,
        proof.sibling_branch.0.elements[1].0,
        proof.sibling_branch.0.elements[2].0,
        proof.sibling_branch.0.elements[3].0,
    ]);
    inputs.extend(vec![
        proof.reward_leaf.0.elements[0].0,
        proof.reward_leaf.0.elements[1].0,
        proof.reward_leaf.0.elements[2].0,
        proof.reward_leaf.0.elements[3].0,
    ]);
    inputs.extend(vec![proof.proof_height.0, proof.index.0]);
}

fn get_job_proof(
    provider: &RpcProvider,
    job_info: &JobInfo,
    checkpoint_id: u64,
) -> anyhow::Result<(u64, qed_core::job::id::VariableHeightRewardMerkleProof)> {
    let (job_proof, actual_checkpoint_id) = match &job_info.location {
        JobLocation::Realm(realm_id) => {
            let (proof, root_job_id) = provider.get_job_proof_from_realm(*realm_id, checkpoint_id, job_info.job_id.get_output_id())?;
            (proof, root_job_id.goal_id)
        }
        JobLocation::Coordinator => {
            let (proof, root_job_id) = provider.get_job_proof_from_coordinator(checkpoint_id, job_info.job_id.get_output_id())?;
            (proof, root_job_id.goal_id)
        }
    };

    Ok((actual_checkpoint_id, job_proof.pad_to_height(GUTA_REWARDS_TREE_MAX_HEIGHT)))
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

fn load_jobs_from_tracker_file(public_key: &QHashOut<F>, target_checkpoint_id: u64) -> Result<Vec<JobInfo>> {
    let filename = format!("{}.json", public_key.to_string());

    if !std::path::Path::new(&filename).exists() {
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
