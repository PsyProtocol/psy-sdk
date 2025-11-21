use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use kvq::traits::KVQSerializable;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    hash::hash_types::HashOut,
    plonk::config::PoseidonGoldilocksConfig,
};
use psy_common::{
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
    job::id::{ProvingJobCircuitType, QProvingJobDataID, VariableHeightRewardMerkleProof, GUTA_REWARDS_TREE_MAX_HEIGHT},
};
use psy_crypto::signature::zk::wallet::SimplePsyPrivateKey;
use psy_data::{config::store_config::PsyHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync};
use psy_node::worker::job_tracker::{JobInfo, JobLocation, WorkerJobTracker};
use psy_prover::session::WalletSession;
use psy_rust_sdk::provider::{NetworkConfig, RpcProvider};
use serde_json::json;
use tracing::info;

use crate::subcommand::GetJobProofArgs;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub async fn run(args: GetJobProofArgs) -> Result<()> {
    info!("Starting get job proof with checkpoint_id: {}", args.checkpoint_id);

    let psy_config = psy_config::PsyConfig::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?;
    let private_key = QHashOut::from(Hash256::from_hex_string(&args.private_key)?);

    let provider = RpcProvider::new_with_config(&rpc_config)?;

    let mut wallet_session = WalletSession::new(&rpc_config).await?;
    let user_pk_hash = wallet_session
        .add_user(private_key, psy_prover::wallet::memory_wallet::get_zk_fingerprint())
        .await?;

    let job_infos = if let Some(job_id_hex) = &args.job_id {
        let job_id = parse_job_id_from_hex(job_id_hex)?;
        vec![JobInfo {
            job_id,
            location: JobLocation::Coordinator,
        }]
    } else {
        load_jobs_from_tracker_file(&user_pk_hash, args.checkpoint_id)?
    };

    info!("Processing {} jobs for checkpoint {}", job_infos.len(), args.checkpoint_id);

    let mut proofs = Vec::new();

    for job_info in &job_infos {
        if job_info.job_id.circuit_type.is_guta_job() {
            // GUTA job - continue processing
        } else {
            info!("Skipping non-GUTA job type: {:?}", job_info.job_id.circuit_type);
            continue;
        }

        info!("Processing job: {:?}", job_info.job_id);

        match get_job_proof(&provider, &job_info, args.checkpoint_id).await {
            Ok(job_proof) => {
                info!("Successfully got proof for job: {}", job_info.job_id.to_hex_string());

                let (verified, root, nullifier_index) = match verify_proof(&job_proof) {
                    Ok((root, nullifier_index)) => (true, Some(root), Some(nullifier_index)),
                    Err(_) => (false, None, None),
                };

                proofs.push((job_info.clone(), job_proof, verified, root, nullifier_index));
            }
            Err(e) => {
                info!(
                    "Skipping job({}) due to error: {:?} - {}",
                    job_info.job_id.to_hex_string(),
                    job_info.job_id,
                    e
                );
                continue;
            }
        }
    }

    if proofs.is_empty() {
        return Err(anyhow::format_err!("No valid proofs found"));
    }

    info!("Successfully processed {} proofs for checkpoint {}", proofs.len(), args.checkpoint_id);

    let json_data: Result<Vec<_>> = proofs
        .iter()
        .map(|(info, proof, verified, root, nullifier_index)| {
            let mut json_obj = serde_json::json!({
                "job_id": info.job_id.to_hex_string(),
                "circuit_type": format!("{:?}", info.job_id.circuit_type),
                "location": format!("{:?}", info.location),
                "verified": verified,
                "proof": proof
            });

            if let Some(root) = root {
                json_obj["root"] = serde_json::json!(format!("{}", root));
            }

            if let Some(nullifier_index) = nullifier_index {
                json_obj["nullifier_index"] = serde_json::json!(nullifier_index.0);
            }

            Ok(json_obj)
        })
        .collect();

    let json_data = json_data?;
    let output = serde_json::to_string_pretty(&json_data)?;
    println!("{}", output);

    Ok(())
}

async fn get_job_proof(
    provider: &RpcProvider,
    job_info: &JobInfo,
    checkpoint_id: u64,
) -> Result<psy_common::job::id::VariableHeightRewardMerkleProof> {
    let results = provider.get_job_proofs(vec![job_info.clone()]).await?;

    if results.is_empty() {
        return Err(anyhow::format_err!("No proof returned for job ID").into());
    }

    let (_, job_proof) = results.into_iter().next().unwrap();

    match &job_info.location {
        JobLocation::Realm(realm_id) => {
            info!("Got realm {} proof: {}", realm_id, serde_json::to_string_pretty(&job_proof).unwrap());
        }
        JobLocation::Coordinator => {
            info!("Got coordinator proof: {}", serde_json::to_string_pretty(&job_proof).unwrap());
        }
    }

    Ok(job_proof)
}

fn verify_proof(proof: &VariableHeightRewardMerkleProof) -> Result<(QHashOut<F>, F)> {
    let (root, nullifier_index) = proof.compute_root_and_nullifier_index();
    Ok((root, nullifier_index))
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
    QProvingJobDataID::try_from_byte_vec(&bytes)
}
