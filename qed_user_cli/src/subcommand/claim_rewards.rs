use std::str::FromStr;
use std::collections::HashMap;

use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::id::{QProvingJobDataID, ProvingJobCircuitType};
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_prover::session::WalletSession;
use qed_prover::local::{
    args::{ContractCallArgs, SignType},
    provider::{RpcConfig, RpcProvider},
};
use tracing::info;
use serde_json::json;
use anyhow::Result;

use super::args::ClaimRewardsArgs;

type F = GoldilocksField;

#[derive(Debug, Clone)]
struct JobInfo {
    job_id: QProvingJobDataID,
    location: JobLocation,
}

#[derive(Debug, Clone)]
enum JobLocation {
    Realm(u64),
    Coordinator,
}

#[derive(Debug, Clone)]
struct JobClaim {
    job_id: u64,
    job_type: u64,
    job_proof: qed_core::job::id::JobProof,
    reward: u64,
}

pub fn run(args: ClaimRewardsArgs) -> Result<()> {
    info!("Starting claim rewards with checkpoint_id: {}", args.checkpoint_id);

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
    let private_key = QHashOut::<F>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("Failed to parse private key: {}", e))?;
    let job_infos = parse_job_specs(&args.jobs)?;
    let provider = RpcProvider::new_with_config(&rpc_config)?;
    let mut wallet_session = WalletSession::new(&rpc_config)?;
    let fingerprint = if args.fingerprint.is_some() {
        Some(QHashOut::<F>::from_str(&args.fingerprint.as_ref().unwrap())
            .map_err(|e| anyhow::format_err!("Failed to parse fingerprint: {}", e))?)
    } else {
        None
    };

    let user_pk_hash = wallet_session.add_user_with_type(
        private_key,
        args.sign_type.clone(),
        fingerprint
    )?;
    let user_public_key_hash = user_pk_hash;
    let mut claims = Vec::new();

    for job_info in &job_infos {
        info!("Processing job: {:?}", job_info.job_id);
        let job_type = match job_info.job_id.circuit_type {
            ProvingJobCircuitType::AppendUserRegistrationTree |
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate |
            ProvingJobCircuitType::GUTAOnlyRegisterUsers |
            ProvingJobCircuitType::GUTARegisterUsers => 0,

            ProvingJobCircuitType::GUTATwoEndCap |
            ProvingJobCircuitType::GUTATwoGUTA |
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA |
            ProvingJobCircuitType::GUTALeftGUTARightEndCap |
            ProvingJobCircuitType::GUTASingleEndCap |
            ProvingJobCircuitType::GUTAVerifyToCap |
            ProvingJobCircuitType::GUTANoChange => 1,

            ProvingJobCircuitType::BatchDeployContracts |
            ProvingJobCircuitType::BatchDeployContractsAggregate => 2,

            _ => {
                return Err(anyhow::format_err!(
                    "Unsupported job type for rewards: {:?}",
                    job_info.job_id.circuit_type
                ));
            }
        };
        let job_proof = get_job_proof(
            &provider,
            &job_info,
            args.checkpoint_id,
        )?;

        info!("Job type: {}, Proof value: {:?}, Proof root: {:?}",
              job_type, job_proof.value, job_proof.root);

        // Debug: Check siblings
        let non_zero_siblings = job_proof.siblings.iter()
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
            for _ in 0..4 { inputs.push(0); } // value
            for _ in 0..160 { inputs.push(0); } // siblings (32 * 5)
            for _ in 0..4 { inputs.push(0); } // root
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

    info!("Successfully submitted rewards claim for {} jobs", claim_count);

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
            let realm_id = parts[2].parse::<u64>()
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
