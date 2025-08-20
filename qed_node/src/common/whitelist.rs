use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use k256::sha2::{Digest, Sha256};
use plonky2::plonk::proof::Proof;
use secp256k1::{Message, PublicKey, Secp256k1, ecdsa};
use tracing::{info, warn, error};
use qed_prover::wallet::secp_sign::{ProofSubmission, SignedRequest, MESSAGE_CLAIM_JOB};

// Verify signature
/// Worker entry in the whitelist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEntry {
    /// Worker ID (wallet_id from QHashOut)
    pub worker_id: String,

    /// Secp256k1 public key in hex format
    pub public_key_hex: String,
}

/// Whitelist configuration file structure
#[derive(Debug, Serialize, Deserialize)]
pub struct WhitelistConfig {
    pub workers: Vec<WorkerEntry>,
}

type WorkerId = String;
type WorkerPublicKey = Vec<u8>;
/// Worker whitelist manager
#[derive(Clone)]
pub struct WorkerWhitelist {
    /// Map of worker_id to public key
    workers: Arc<RwLock<HashMap<WorkerId, WorkerPublicKey>>>,

    /// Optional path to persist updates
    config_path: Option<String>,
}

impl WorkerWhitelist {
    /// Create a new empty whitelist
    pub fn new(config_path: Option<String>) -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            config_path,
        }
    }

    /// Load whitelist from a configuration file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        info!("Loading worker whitelist from: {}", path_str);

        let contents = fs::read_to_string(&path)?;
        let config: WhitelistConfig = serde_json::from_str(&contents)?;

        let mut workers = HashMap::new();

        for entry in config.workers {

            // Parse the public key
            match parse_public_key(&entry.public_key_hex) {
                Ok(public_key) => {
                    workers.insert(
                        entry.worker_id.clone(),
                        public_key,
                    );
                    info!("Loaded worker: {}", entry.worker_id);
                }
                Err(e) => {
                    error!("Failed to parse public key for worker {}: {}", entry.worker_id, e);
                }
            }
        }

        Ok(Self {
            workers: Arc::new(RwLock::new(workers)),
            config_path: Some(path_str),
        })
    }

    /// Get a worker's public key
    pub async fn get_worker_pk(&self, worker_id: &str) -> Option<WorkerPublicKey> {
        let workers = self.workers.read().await;
        workers.get(worker_id).cloned()
    }

    /// Check if a worker is whitelisted
    pub async fn is_whitelisted(&self, worker_id: &str) -> bool {
        let workers = self.workers.read().await;
        workers.contains_key(worker_id)
    }

    /// Add a new worker to the whitelist
    pub async fn add_worker(
        &self,
        worker_id: String,
        public_key_hex: String,
    ) -> Result<()> {
        let public_key = parse_public_key(&public_key_hex)?;

        let mut workers = self.workers.write().await;
        workers.insert(
            worker_id.clone(),
            public_key,
        );

        info!("Added worker to whitelist: {}", worker_id);

        // Optionally persist to file
        if let Some(ref path) = self.config_path {
            self.save_to_file_internal(path, &workers).await?;
        }

        Ok(())
    }

    /// Remove a worker from the whitelist
    pub async fn remove_worker(&self, worker_id: &str) -> Result<bool> {
        let mut workers = self.workers.write().await;
        let removed = workers.remove(worker_id).is_some();

        if removed {
            info!("Removed worker from whitelist: {}", worker_id);

            // Optionally persist to file
            if let Some(ref path) = self.config_path {
                self.save_to_file_internal(path, &workers).await?;
            }
        }

        Ok(removed)
    }

    /// Get all whitelisted worker IDs
    pub async fn list_workers(&self) -> Vec<String> {
        let workers = self.workers.read().await;
        workers.keys().cloned().collect()
    }

    /// Get worker count
    pub async fn worker_count(&self) -> usize {
        let workers = self.workers.read().await;
        workers.len()
    }

    /// Reload whitelist from file
    pub async fn reload(&self) -> Result<()> {
        if let Some(ref path) = self.config_path {
            info!("Reloading whitelist from: {}", path);
            let new_whitelist = Self::from_file(path).await?;

            let mut workers = self.workers.write().await;
            let new_workers = new_whitelist.workers.read().await;
            *workers = new_workers.clone();

            info!("Whitelist reloaded with {} workers", workers.len());
        } else {
            warn!("No config path set, cannot reload whitelist");
        }

        Ok(())
    }

    /// Save current whitelist to file
    async fn save_to_file_internal(
        &self,
        path: &str,
        workers: &HashMap<String, WorkerPublicKey>,
    ) -> Result<()> {
        let entries: Vec<WorkerEntry> = workers
            .iter()
            .map(|(id, info)| WorkerEntry {
                worker_id: id.clone(),
                public_key_hex: hex::encode(info),
            })
            .collect();

        let config = WhitelistConfig {
            workers: entries,
        };

        let json = serde_json::to_string_pretty(&config)?;
        fs::write(path, json)?;

        Ok(())
    }


    /// Verify a signed request
    pub async fn verify_request<T: Serialize>(
        &self,
        request: &SignedRequest<T>,
        max_age_seconds: Option<u64>,  // Optional: how old can the request be
    ) -> Result<()> {

        let worker_id = &request.worker_id;
        // Get worker's public key from whitelist
        let worker_pk = self
            .get_worker_pk(worker_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Worker not whitelisted: {}", worker_id))?;

        //Check timestamp freshness (if max_age is specified)
        if let Some(max_age) = max_age_seconds {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();

            if current_time > request.timestamp &&
                current_time - request.timestamp > max_age {
                return Err(anyhow::anyhow!(
                    "Request expired: {} seconds old (max: {} seconds)",
                    current_time - request.timestamp,
                    max_age
                ));
            }

            // Also check for future timestamps (clock skew protection)
            if request.timestamp > current_time + 60 {  // Allow 60 seconds clock skew
                return Err(anyhow::anyhow!(
                    "Request timestamp is in the future"
                ));
            }
        }

        // 2. Recreate the exact same message that was signed
        let message_content = serde_json::json!({
            "data": &request.data,
            "worker_id": &request.worker_id,
            "timestamp": request.timestamp,
        });

        let message_bytes = serde_json::to_vec(&message_content)?;

        // 3. Hash the message (same as in sign method)
        let mut hasher = Sha256::new();
        hasher.update(&message_bytes);
        let hash = hasher.finalize();

        // 4. Decode the signature
        let signature_bytes = hex::decode(&request.signature)?;

        // 5. Verify the signature
        let secp = Secp256k1::new();
        let message = Message::from_digest_slice(&hash)?;
        let signature = ecdsa::Signature::from_compact(&signature_bytes)?;
        let public_key = PublicKey::from_slice(&worker_pk)
            .map_err(|_| anyhow::anyhow!("Invalid public key format"))?;
        secp.verify_ecdsa(message, &signature, &public_key)
            .map_err(|_| anyhow::anyhow!("Invalid signature"))?;

        Ok(())
    }

    pub async fn verify_claim_job(&self, signed: &SignedRequest<String>) -> Result<()> {
        let msg = signed.data.clone();
        if msg != MESSAGE_CLAIM_JOB {
            return Err(anyhow::anyhow!("Invalid claim job message: {}", msg));
        }
        self.verify_request(
            &signed,
            Some(30),
        ).await
    }
    pub async fn verify_submit_proof(&self, signed: &SignedRequest<ProofSubmission>,) -> Result<()> {
        //verify the proof hash
        let proof = &signed.data.proof;
        let expected_proof_hash = &ProofSubmission::calculate_optional_proof_hash(proof)?;
        let real_proof_hash = &signed.data.proof_hash;
        if expected_proof_hash != real_proof_hash {
            return Err(anyhow::anyhow!(
                "Proof hash mismatch: expected {}, got {}",
                expected_proof_hash,
                real_proof_hash
            ));
        }

        //verify the signature and worker id
        self.verify_request(
            &signed,
            Some(30),
        ).await

    }
}

/// Parse a hex-encoded public key
fn parse_public_key(hex_str: &str) -> Result<WorkerPublicKey> {
    let bytes = if hex_str.starts_with("0x") {
        hex::decode(&hex_str[2..])?
    } else {
        hex::decode(hex_str)?
    };
    Ok(bytes)
}

