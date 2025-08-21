use anyhow::{Context, Result};
use k256::sha2::{Digest, Sha256};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_store::queue::task_queue::QJob;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

use crate::wallet::secp_wallet::Wallet;

/// Default signature expiry duration (5 minutes)
const DEFAULT_SIGNATURE_EXPIRY: Duration = Duration::from_secs(300);

/// Message type for job claims
pub const MESSAGE_CLAIM_JOB: &str = "claim_job";

/// Timestamp provider trait for testing
pub trait TimestampProvider: Send + Sync {
    fn now(&self) -> u64;
}

/// Default system time provider
#[derive(Clone, Debug)]
pub struct SystemTimeProvider;

impl TimestampProvider for SystemTimeProvider {
    fn now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

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
        Self::new_with_timestamp(wallet, data, SystemTimeProvider.now())
    }
    /// Create with specific timestamp (for testing)
    pub fn new_with_timestamp(wallet: &Wallet, data: T, timestamp: u64) -> Result<Self> {
        let message = create_message(&data, &wallet.id(), timestamp);
        let signature = sign_message(wallet, &message)?;

        Ok(Self {
            data,
            worker_id: wallet.id(),
            signature,
            timestamp,
        })
    }
    /// Verify signature and optionally check expiry
    pub fn verify(&self, wallet: &Wallet, check_expiry: bool) -> Result<bool> {
        if check_expiry && self.is_expired() {
            return Ok(false);
        }

        let message = create_message(&self.data, &self.worker_id, self.timestamp);
        let signature_bytes = hex::decode(&self.signature)?;

        Wallet::verify_signature(
            &serde_json::to_vec(&message)?,
            &signature_bytes,
            wallet.address_raw(),
        )
    }

    /// Check if request has expired (default 5 minutes)
    pub fn is_expired(&self) -> bool {
        self.is_expired_with_duration(DEFAULT_SIGNATURE_EXPIRY)
    }

    /// Check if request has expired with custom duration
    pub fn is_expired_with_duration(&self, duration: Duration) -> bool {
        let now = SystemTimeProvider.now();
        now > self.timestamp + duration.as_secs()
    }

    /// Get the age of the request in seconds
    pub fn age(&self) -> u64 {
        SystemTimeProvider.now().saturating_sub(self.timestamp)
    }
}

impl<T: Display + Serialize> Display for SignedRequest<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignedRequest[worker={}, age={}s]",
               &self.worker_id[..8], self.age())
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
        let proof_hash = proof.as_ref()
            .map(Self::hash_proof)
            .transpose()?
            .unwrap_or_default();
        Ok(Self { job, proof, proof_hash })
    }

    /// Create submission without proof (for failures)
    pub fn failure(job: QJob) -> Self {
        Self {
            job,
            proof: None,
            proof_hash: String::new(),
        }
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
    /// Check if submission contains a proof
    pub fn has_proof(&self) -> bool {
        self.proof.is_some()
    }
}

impl fmt::Display for ProofSubmission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProofSubmission[job_id={:?}, has_proof={}, hash={}...]",
               self.job.job_id, self.has_proof(), &self.proof_hash[..8])
    }
}

/// Builder for creating various signed requests
pub struct RequestBuilder<'a> {
    wallet: &'a Wallet,
    timestamp: Option<u64>,
}

impl<'a> RequestBuilder<'a> {
    /// Create new builder
    pub fn new(wallet: &'a Wallet) -> Self {
        Self {
            wallet,
            timestamp: None,
        }
    }

    /// Set custom timestamp
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Build job claim request
    pub fn claim_job(self) -> Result<SignedRequest<String>> {
        self.build(MESSAGE_CLAIM_JOB.to_string())
    }

    /// Build proof submission request
    pub fn submit_proof(self, job: QJob, proof: Option<ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>>) -> Result<SignedRequest<ProofSubmission>> {
        let submission = ProofSubmission::new(job, proof)?;
        self.build(submission)
    }

    /// Build custom request
    pub fn custom<T: Serialize>(self, data: T) -> Result<SignedRequest<T>> {
        self.build(data)
    }

    /// Internal build method
    fn build<T: Serialize>(self, data: T) -> Result<SignedRequest<T>> {
        match self.timestamp {
            Some(ts) => SignedRequest::new_with_timestamp(self.wallet, data, ts),
            None => SignedRequest::new(self.wallet, data),
        }
    }
}


// Convenience methods for specific request types
impl Wallet {
    /// Create a request builder
    pub fn request(&self) -> RequestBuilder {
        RequestBuilder::new(self)
    }

    /// Quick method to create job claim
    pub fn sign_claim_job(&self) -> Result<SignedRequest<String>> {
        self.request().claim_job()
    }

    /// Quick method to submit proof
    pub fn sign_proof_submission(
        &self,
        job: QJob,
        proof: Option<ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>>,
    ) -> Result<SignedRequest<ProofSubmission>> {
        self.request().submit_proof(job, proof)
    }
}


fn create_message<T: Serialize>(data: &T, worker_id: &str, timestamp: u64) -> serde_json::Value {
    serde_json::json!({
        "data": data,
        "worker_id": worker_id,
        "timestamp": timestamp,
    })
}

fn sign_message(wallet: &Wallet, message: &serde_json::Value) -> Result<String> {
    let message_bytes = serde_json::to_vec(message)?;
    let signature = wallet.sign_raw(&message_bytes)?;
    Ok(hex::encode(signature))
}
