use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};
use maybe_async::maybe_async;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    hash::{
        hash_types::RichField,
        hashing::{hash_n_to_hash_no_pad, PlonkyPermutation},
    },
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_core::{
    data::{
        qhashout::QHashOut,
        secp256k1::{bytes_to_u32_vec_le, CompressedPublicKey},
    },
    job::id::{ProvingJobCircuitType, QProvingJobDataID, GUTA_REWARDS_TREE_MAX_HEIGHT},
};
use qed_crypto::signature::secp256k1::core::QEDCompressedSecp256K1Signature;
use serde::{Deserialize, Serialize};

use crate::local::{
    args::{JobInfo, JobLocation, WorkerJobTracker},
    provider::RpcProvider,
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub fn hash_no_pad_compressed_public_key<F: RichField, P: PlonkyPermutation<F>>(secp256k1_public_key: CompressedPublicKey) -> QHashOut<F> {
    let mut secp256k1_public_key_bytes = vec![secp256k1_public_key.0[0], 0, 0, 0];
    secp256k1_public_key_bytes.extend_from_slice(&secp256k1_public_key.0[1..]);
    let secp256k1_public_key_f = bytes_to_u32_vec_le(&secp256k1_public_key_bytes)
        .iter()
        .map(|n| F::from_canonical_u32(*n))
        .collect::<Vec<_>>();

    QHashOut(hash_n_to_hash_no_pad::<F, P>(&secp256k1_public_key_f))
}

pub fn secp256k1_sign<F: RichField>(private_key: SigningKey, sighash: QHashOut<F>) -> anyhow::Result<QEDCompressedSecp256K1Signature> {
    tracing::info!("🔔 prove_secp256k1_signature");

    // let private_key: Hash256 = private_key.into();
    // let private_key = k256::ecdsa::SigningKey::from_slice(&private_key.0)?;
    let public_key = private_key.verifying_key().to_encoded_point(true).to_bytes();
    let mut compressed = [0u8; 33];
    if public_key.len() == 33 {
        compressed.copy_from_slice(&public_key);
    } else {
        return Err(anyhow::format_err!("pub key length is not 33"));
    }
    let pub_compressed = CompressedPublicKey(compressed);
    let result: k256::ecdsa::Signature = private_key.sign_prehash(&sighash.to_le_bytes())?;
    let mut rs_bytes = [0u8; 64];

    let r_bytes = result.r().to_bytes();
    let s_bytes = result.s().to_bytes();
    rs_bytes[0..32].copy_from_slice(&r_bytes);
    rs_bytes[32..64].copy_from_slice(&s_bytes);

    Ok(QEDCompressedSecp256K1Signature {
        public_key: pub_compressed.0,
        signature: rs_bytes,
        message: sighash.into(),
    })
}

#[maybe_async]
pub async fn parse_job_specs(specs: &[String]) -> anyhow::Result<Vec<JobInfo>> {
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

#[maybe_async]
pub async fn get_job_proof(
    provider: &RpcProvider,
    job_info: &JobInfo,
    checkpoint_id: u64,
) -> anyhow::Result<(u64, qed_core::job::id::VariableHeightRewardMerkleProof)> {
    let (job_proof, actual_checkpoint_id) = match &job_info.location {
        JobLocation::Realm(realm_id) => {
            let (proof, root_job_id) = provider
                .get_job_proof_from_realm(*realm_id, checkpoint_id, job_info.job_id.get_output_id())
                .await?;
            (proof, root_job_id.goal_id)
        }
        JobLocation::Coordinator => {
            let (proof, root_job_id) = provider
                .get_job_proof_from_coordinator(checkpoint_id, job_info.job_id.get_output_id())
                .await?;
            (proof, root_job_id.goal_id)
        }
    };

    Ok((actual_checkpoint_id, job_proof.pad_to_height(GUTA_REWARDS_TREE_MAX_HEIGHT)))
}

#[maybe_async]
pub async fn load_jobs_from_tracker_file(public_key: &QHashOut<F>, target_checkpoint_id: u64) -> anyhow::Result<Vec<JobInfo>> {
    let filename = format!("{}.json", public_key.to_string());

    if !std::path::Path::new(&filename).exists() {
        tracing::info!("No job tracker file found: {}", filename);
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&filename)?;
    let tracker: WorkerJobTracker = serde_json::from_str(&content)?;

    let mut job_infos = Vec::new();

    if let Some(coordinator_jobs) = tracker.coordinator.get(&target_checkpoint_id) {
        for job_hex in coordinator_jobs {
            let job_id = parse_job_id_from_hex(job_hex).await?;
            job_infos.push(JobInfo {
                job_id,
                location: JobLocation::Coordinator,
            });
        }
    }

    for realm in &tracker.realms {
        if let Some(realm_jobs) = realm.checkpoints.get(&target_checkpoint_id) {
            for job_hex in realm_jobs {
                let job_id = parse_job_id_from_hex(job_hex).await?;
                job_infos.push(JobInfo {
                    job_id,
                    location: JobLocation::Realm(realm.id as u64),
                });
            }
        }
    }

    tracing::info!(
        "Loaded {} jobs from tracker file {} for checkpoint {}",
        job_infos.len(),
        filename,
        target_checkpoint_id
    );
    Ok(job_infos)
}

#[maybe_async]
pub async fn parse_job_id_from_hex(hex_str: &str) -> anyhow::Result<QProvingJobDataID> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(hex_str)?;
    if bytes.len() != 24 {
        anyhow::bail!("Invalid job ID length: expected 24 bytes, got {}", bytes.len());
    }
    QProvingJobDataID::try_from_byte_vec(&bytes)
}
