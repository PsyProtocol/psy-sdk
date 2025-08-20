use std::fs;
use std::path::Path;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use alloy_primitives::Address;
use alloy_signer_local::PrivateKeySigner;
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
        let signature = hex::encode(wallet.sign_raw(&message_bytes)?);

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
        SignedRequest::new(self, "claim_job".to_string())
    }

    /// Create a proof submission request
    pub fn sign_proof_submission(
        &self,
        job: QJob,
        proof: Option<ProofWithPublicInputs<
            GoldilocksField,
            PoseidonGoldilocksConfig,
            2
        >>
    ) -> Result<SignedRequest<ProofSubmission>> {
        let submission = ProofSubmission::new(job, proof)?;
        SignedRequest::new(self, submission)
    }
    /// List all accounts in a keystore directory (following Foundry's list pattern)
    pub fn list_accounts(dir: Option<&Path>) -> Result<Vec<String>> {
        let keystore_dir = dir
            .map(|p| p.to_path_buf())
            .or_else(|| dirs::home_dir().map(|h| h.join(".foundry/keystores")))
            .ok_or_else(|| anyhow::anyhow!("Could not determine keystore directory"))?;

        let mut accounts = Vec::new();
        if keystore_dir.exists() {
            for entry in fs::read_dir(keystore_dir)? {
                let path = entry?.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        accounts.push(name.to_string());
                    }
                }
            }
        }
        Ok(accounts)
    }

    /// Generate a vanity address (following Foundry's vanity pattern)
    pub fn vanity(starts_with: Option<&str>, ends_with: Option<&str>) -> Result<Self> {
        use rayon::iter::{IntoParallelIterator, ParallelIterator};

        let matcher = |addr: &Address| -> bool {
            let hex_addr = hex::encode(addr.as_slice());
            let start_match = starts_with.map_or(true, |s| hex_addr.starts_with(s));
            let end_match = ends_with.map_or(true, |s| hex_addr.ends_with(s));
            start_match && end_match
        };

        // Generate wallets in parallel until we find a match
        let wallet = (0..u64::MAX)
            .into_par_iter()
            .map(|_| {
                let mut rng = rand::thread_rng();
                PrivateKeySigner::random_with(&mut rng)
            })
            .find_any(|w| matcher(&w.address()))
            .ok_or_else(|| anyhow::anyhow!("Failed to generate vanity address"))?;

        Self::from_private_key_signer(wallet)
    }
}