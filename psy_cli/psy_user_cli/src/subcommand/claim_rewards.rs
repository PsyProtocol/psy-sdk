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
use psy_services::models::{WorkerEvent, WorkerEventSource};
use psy_common_circuit::circuits::zk_signature3::manager::SimplePsyZKSignatureManager;
use psy_core::{
    config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, TOKEN_CONTRACT_ID},
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
    job::id::{ProvingJobCircuitType, QProvingJobDataID, VariableHeightRewardMerkleProof, GUTA_REWARDS_TREE_MAX_HEIGHT},
};
use psy_crypto::signature::zk::wallet::SimplePsyPrivateKey;
use psy_data::{
    config::store_config::PsyHasher,
    traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync},
};
use psy_prover::{
    local::args::{ContractCallArgs, SignData, SignType},
    session::WalletSession,
};
use psy_rust_sdk::provider::{JobInfo, JobLocation, RpcConfig, RpcProvider};
use psy_node::worker::job_tracker::WorkerJobTracker;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

use super::args::ClaimRewardsArgs;

type ApiResponse = Vec<WorkerEvent>;

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

pub async fn run(args: ClaimRewardsArgs) -> Result<()> {
    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
    let private_key = QHashOut::from(Hash256::from_hex_string(&args.private_key)?);

    let provider = RpcProvider::new_with_config(&rpc_config)?;
    let mut wallet_session = WalletSession::new(&rpc_config).await?;
    let fingerprint = if args.fingerprint.is_some() {
        Some(QHashOut::<F>::from_str(&args.fingerprint.as_ref().unwrap()).map_err(|e| anyhow::format_err!("Failed to parse fingerprint: {}", e))?)
    } else {
        None
    };

    let user_pk_hash = wallet_session
        .add_user_with_type(private_key, args.sign_type.clone(), fingerprint)
        .await?;
    let user_id = provider.get_user_id(user_pk_hash).await?;

    let latest_block_state = provider.get_latest_block_state().await?;
    let latest_checkpoint_id = latest_block_state.checkpoint_id;

    info!("Latest checkpoint: {}", latest_checkpoint_id);

    let start_checkpoint = if let Some(start_checkpoint_id) = args.start_checkpoint_id {
        info!("Using manually specified start checkpoint: {}", start_checkpoint_id);
        start_checkpoint_id
    } else {
        let last_claimed = match get_last_claimed_checkpoint_id(&provider, user_id, latest_checkpoint_id).await {
            Ok(checkpoint) => {
                info!("Last claimed checkpoint: {}", checkpoint);
                checkpoint
            }
            Err(e) => {
                warn!("Failed to query last claimed checkpoint ({}), starting from checkpoint 1", e);
                0
            }
        };
        last_claimed + 1
    };

    let claim_rewards_cooldown = 0;
    let max_claimable_checkpoint = if latest_checkpoint_id > claim_rewards_cooldown {
        latest_checkpoint_id - claim_rewards_cooldown
    } else {
        0
    };

    if start_checkpoint > max_claimable_checkpoint {
        info!(
            "No new checkpoints to claim (latest: {}, cooldown: {}, max claimable: {})",
            latest_checkpoint_id, claim_rewards_cooldown, max_claimable_checkpoint
        );
        return Ok(());
    }

    let mut checkpoint_jobs: HashMap<u64, Vec<VariableHeightRewardMerkleProof>> = HashMap::new();
    let mut processed_count = 0;

    info!(
        "Claiming rewards from checkpoint {} to {} (limit: {}, latest: {}, cooldown: {})",
        start_checkpoint, max_claimable_checkpoint, args.limit, latest_checkpoint_id, claim_rewards_cooldown
    );

    let all_job_infos = if !args.jobs.is_empty() {
        info!("Using manually specified jobs from args.jobs");
        let jobs = parse_job_specs(&args.jobs)?;
        let mut all_jobs = HashMap::new();
        for job in jobs {
            let checkpoint_id = job.job_id.goal_id;
            if checkpoint_id >= start_checkpoint && checkpoint_id <= max_claimable_checkpoint {
                all_jobs.entry(checkpoint_id).or_insert_with(Vec::new).push(job);
            }
        }
        info!("Loaded {} checkpoints from manual job specs", all_jobs.len());
        all_jobs
    } else if args.api_service_url.is_empty() {
        info!("Loading jobs from file (api_service_url is empty)");
        let all_jobs_from_file = load_jobs_from_tracker_file(&user_pk_hash)?;
        let mut all_jobs = HashMap::new();
        for job in all_jobs_from_file {
            let checkpoint_id = job.job_id.goal_id;
            if checkpoint_id >= start_checkpoint && checkpoint_id <= max_claimable_checkpoint {
                all_jobs.entry(checkpoint_id).or_insert_with(Vec::new).push(job);
            }
        }
        all_jobs
    } else {
        info!("Using API service at {}", args.api_service_url);
        let result = load_jobs_from_api_service(
            &args.api_service_url,
            &user_pk_hash,
            start_checkpoint,
            max_claimable_checkpoint,
            args.limit,
        )
        .await?;
        result
    };

    info!("all_job_infos: {}", serde_json::to_string_pretty(&all_job_infos).unwrap());

    let mut sorted_checkpoints: Vec<_> = all_job_infos.keys().copied().collect();
    sorted_checkpoints.sort();

    for checkpoint_id in sorted_checkpoints {
        if processed_count >= args.limit {
            break;
        }

        let mut job_infos = all_job_infos.get(&checkpoint_id).cloned().unwrap_or_default();
        job_infos.retain(|job_info| {
            matches!(
                job_info.job_id.circuit_type,
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
                    | ProvingJobCircuitType::GUTANoChange
            )
        });
        if job_infos.is_empty() {
            continue;
        }
        info!("Checkpoint {} - Found {} valid jobs", checkpoint_id, job_infos.len());

        match provider.get_job_proofs(job_infos.clone()).await {
            Ok(results) => {
                for (root_job_id, job_proof) in results {
                    let actual_checkpoint_id = root_job_id.goal_id;
                    checkpoint_jobs
                        .entry(actual_checkpoint_id)
                        .or_insert_with(Vec::new)
                        .push((job_proof.clone().pad_to_height(GUTA_REWARDS_TREE_MAX_HEIGHT)));
                }
            }
            Err(e) => {
                warn!("Failed to get job proofs for checkpoint {}: {}", checkpoint_id, e);
            }
        }
        processed_count += 1;
    }

    info!(
        "Total jobs attempted: processed {} checkpoints, checkpoint_jobs.len() = {}",
        processed_count,
        checkpoint_jobs.len()
    );
    if checkpoint_jobs.is_empty() {
        info!("No valid checkpoints with rewards to claim - all job proofs failed");
        return Ok(());
    }

    let mut sorted_checkpoints: Vec<_> = checkpoint_jobs.keys().copied().collect();
    sorted_checkpoints.sort();

    let mut all_proofs_with_checkpoints = Vec::new();
    for &checkpoint_id in &sorted_checkpoints {
        let jobs = checkpoint_jobs.get(&checkpoint_id).unwrap();
        let checkpoint_leaf = provider.get_checkpoint_leaf_data(checkpoint_id).await?;
        let fees_collected = checkpoint_leaf.stats.fees_collected.to_canonical_u64();
        let gutas_completed = checkpoint_leaf.stats.pm_jobs_completed.gutas_completed.to_canonical_u64();
        let proposed_reward = if gutas_completed > 0 { fees_collected / gutas_completed } else { 0u64 };
        if proposed_reward == 0 {
            warn!(
                "Skipping checkpoint {} due to zero reward (fees_collected={}, gutas_completed={})",
                checkpoint_id, fees_collected, gutas_completed
            );
            continue;
        }
        info!("Checkpoint {} - Reward: {}, Jobs: {}", checkpoint_id, proposed_reward, jobs.len());
        for proof in jobs {
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
    let last_checkpoint = all_proofs_with_checkpoints.last().unwrap().checkpoint_id;
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
    let sign_data = fingerprint.map(|fp| SignData {
        fingerprint: fp,
        sign_contract_id: MINING_REWARDS_CONTRACT_ID,
        sign_inputs: vec![],
    });
    let tx_hash = wallet_session
        .exec_contract_call_with_sign_data(user_pk_hash, all_contract_calls, sign_data)
        .await?;
    info!("Successfully claimed rewards with tx hash: {}", tx_hash);
    Ok(())
}

async fn get_last_claimed_checkpoint_id(provider: &RpcProvider, user_id: u64, latest_checkpoint_id: u64) -> Result<u64> {
    let proof = provider
        .get_user_contract_state_tree_merkle_proof(
            latest_checkpoint_id,
            user_id,
            TOKEN_CONTRACT_ID,
            MAX_CONTRACT_STATE_TREE_HEIGHT,
            LAST_CLAIMED_CHECKPOINT_SLOT,
        )
        .await?;

    Ok(proof.value.0.elements[1].0)
}

fn build_claim_calls_for_multi_checkpoints(all_proofs: &[ProofWithCheckpoint]) -> Vec<ContractCallArgs> {
    let mut contract_call_args = Vec::new();

    let total_proofs = all_proofs.len();
    let mut proof_index = 0;

    let count_5s = total_proofs / 5;
    let mut remaining = total_proofs % 5;

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

    let count_2s = remaining / 2;
    for _ in 0..count_2s {
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
    remaining = remaining % 2;

    if remaining > 0 {
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
    tracing::debug!("🔍 Serializing proof: {}", serde_json::to_string_pretty(proof).unwrap());

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

async fn load_jobs_from_api_service(
    api_url: &str,
    public_key: &QHashOut<F>,
    from_checkpoint_id: u64,
    to_checkpoint_id: u64,
    limit: usize,
) -> Result<HashMap<u64, Vec<JobInfo>>> {
    let public_key_hex = public_key.to_string();
    let url = format!(
        "{}/worker_events?public_key={}&from_checkpoint_id={}&to_checkpoint_id={}&limit={}&category=reward_only",
        api_url, public_key_hex, from_checkpoint_id, to_checkpoint_id, limit
    );

    info!("Fetching worker events from API: {}", url);

    let response = reqwest::get(&url).await?;
    let api_response: ApiResponse = response.json().await?;

    let mut checkpoint_jobs: HashMap<u64, Vec<JobInfo>> = HashMap::new();

    for event in api_response {
        let job_id = event.job_id;
        let location = match event.source {
            WorkerEventSource::Coordinator => JobLocation::Coordinator,
            WorkerEventSource::Realm => JobLocation::Realm(event.realm_id.unwrap_or(0) as u64),
        };

        let job_info = JobInfo { job_id, location };

        checkpoint_jobs.entry(event.checkpoint_id as u64).or_insert_with(Vec::new).push(job_info);
    }

    info!("Loaded jobs from API for {} checkpoints", checkpoint_jobs.len());
    Ok(checkpoint_jobs)
}

fn load_jobs_from_tracker_file(public_key: &QHashOut<F>) -> Result<Vec<JobInfo>> {
    let filename = format!("{}.json", public_key.to_string());

    if !std::path::Path::new(&filename).exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&filename)?;

    if let Ok(tracker) = serde_json::from_str::<WorkerJobTracker>(&content) {
        info!("Parsing worker job tracker format");
        return parse_worker_job_tracker_format(tracker);
    }

    if let Ok(events) = serde_json::from_str::<Vec<WorkerEvent>>(&content) {
        info!("Parsing worker event format");
        return parse_worker_event_format(events);
    }

    if let Ok(checkpoint_jobs_tuples) = serde_json::from_str::<Vec<(u64, Vec<JobInfo>)>>(&content) {
        info!("Parsing job info format");
        return parse_job_info_format(checkpoint_jobs_tuples);
    }

    Err(anyhow::format_err!("Failed to parse job file in any known format"))
}

fn parse_worker_job_tracker_format(tracker: WorkerJobTracker) -> Result<Vec<JobInfo>> {
    let mut job_infos = Vec::new();

    for (_checkpoint_id, coordinator_jobs) in tracker.coordinator {
        for job_hex in coordinator_jobs {
            let job_id = parse_job_id_from_hex(&job_hex)?;
            job_infos.push(JobInfo {
                job_id,
                location: JobLocation::Coordinator,
            });
        }
    }

    for realm in &tracker.realms {
        for (_checkpoint_id, realm_jobs) in &realm.checkpoints {
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

fn parse_worker_event_format(events: Vec<WorkerEvent>) -> Result<Vec<JobInfo>> {
    let mut job_infos = Vec::new();

    for event in events {
        let job_id = event.job_id;
        let location = match event.source {
            WorkerEventSource::Coordinator => JobLocation::Coordinator,
            WorkerEventSource::Realm => JobLocation::Realm(event.realm_id.unwrap_or(0) as u64),
        };

        job_infos.push(JobInfo { job_id, location });
    }

    Ok(job_infos)
}

fn parse_job_info_format(checkpoint_jobs_tuples: Vec<(u64, Vec<JobInfo>)>) -> Result<Vec<JobInfo>> {
    let mut all_jobs = Vec::new();
    for (_checkpoint_id, jobs) in checkpoint_jobs_tuples {
        all_jobs.extend(jobs);
    }
    Ok(all_jobs)
}

fn parse_job_id_from_hex(hex_str: &str) -> Result<QProvingJobDataID> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(hex_str)?;
    QProvingJobDataID::try_from_byte_vec(&bytes)
}
