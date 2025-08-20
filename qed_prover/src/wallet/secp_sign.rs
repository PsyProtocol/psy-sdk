use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_store::queue::task_queue::QJob;
use k256::sha2::{Digest, Sha256};
use crate::wallet::secp_wallet::Wallet;

pub const MESSAGE_CLAIM_JOB: &str = "claim_job";

/// Signed request wrapper for authenticated RPC calls
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedRequest<T> {
    pub data: T,
    pub worker_id: String,
    pub signature: String,
    pub timestamp: u64,
}

impl<T: Serialize> SignedRequest<T> {
    /// Create a new signed request
    pub fn new(wallet: &Wallet, data: T) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let message = serde_json::json!({
            "data": &data,
            "worker_id": wallet.id(),
            "timestamp": timestamp,
        });

        let message_bytes = serde_json::to_vec(&message)?;
        let signature = hex::encode(wallet.sign(&message_bytes)?);

        Ok(Self {
            data,
            worker_id: wallet.id(),
            signature,
            timestamp,
        })
    }
}

/// Proof submission structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProofSubmission {
    pub job: QJob,
    pub proof: Option<ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>>,
    pub proof_hash: String,
}

impl ProofSubmission {
    /// Create a new proof submission
    pub fn new(
        job: QJob,
        proof: Option<ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>>
    ) -> Result<Self> {
        let proof_hash = Self::calculate_optional_proof_hash(&proof)?;
        Ok(Self { job, proof, proof_hash })
    }

    /// Calculate deterministic hash of a proof
    fn hash_proof(
        proof: &ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>
    ) -> Result<String> {
        let proof_bytes = bincode::serialize(proof)?;
        Ok(hex::encode(Sha256::digest(&proof_bytes)))
    }
    /// Calculate hash for optional proof
    pub fn calculate_optional_proof_hash(
        proof: &Option<ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>>
    ) -> Result<String> {
        proof.as_ref()
            .map(|p| Self::hash_proof(p))
            .transpose()
            .map(|h| h.unwrap_or_default())
    }
}

// Convenience methods for specific request types
impl Wallet {
    /// Create a job claim request
    pub fn sign_claim_job(&self) -> Result<SignedRequest<String>> {
        self.sign_request(MESSAGE_CLAIM_JOB.to_string())
    }

    /// Create a proof submission request
    pub fn sign_proof_submission(
        &self,
        job: QJob,
        proof: Option<ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>>
    ) -> Result<SignedRequest<ProofSubmission>> {
        let submission = ProofSubmission::new(job, proof)?;
        self.sign_request(submission)
    }
}