use anyhow::{Context, Result};
use secp256k1::{Secp256k1, SecretKey, PublicKey, Message};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use k256::sha2::{Digest, Sha256};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::PoseidonPermutation;
use qed_core::data::qhashout::QHashOut;
use qed_core::data::secp256k1::CompressedPublicKey;
use crate::wallet::utils::hash_no_pad_compressed_public_key;
use std::time::{SystemTime, UNIX_EPOCH};

// Re-export for convenience
pub use crate::wallet::keystore::{decode, encode};
use crate::wallet::secp_sign::SignedRequest;

#[derive(Clone, Debug)]
pub struct Wallet {
    secret_key: SecretKey,
    public_key: PublicKey,
    wallet_id: QHashOut<GoldilocksField>,
    secp: Secp256k1<secp256k1::All>,
}

impl Wallet {
    /// Create a new wallet with a random private key
    pub fn new() -> anyhow::Result<Self> {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::rng());
        Self::from_secret_key(secret_key, secp)
    }
    /// Create wallet from existing secret key bytes
    pub fn from_bytes(key_bytes: &[u8]) -> Result<Self> {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(key_bytes)
            .context("Invalid secret key bytes")?;
        Self::from_secret_key(secret_key, secp)
    }

    /// Internal constructor from secret key
    fn from_secret_key(secret_key: SecretKey, secp: Secp256k1<secp256k1::All>) -> Result<Self> {
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let compressed = CompressedPublicKey(public_key.serialize());

        let wallet_id = hash_no_pad_compressed_public_key::<
            GoldilocksField,
            PoseidonPermutation<GoldilocksField>
        >(compressed);

        tracing::debug!("Created wallet with ID: {}", wallet_id);

        Ok(Self {
            secret_key,
            public_key,
            wallet_id,
            secp,
        })
    }

    /// Get wallet ID as string
    pub fn id(&self) -> String {
        self.wallet_id.to_string()
    }

    /// Get wallet ID as QHashOut
    pub fn id_hash(&self) -> QHashOut<GoldilocksField> {
        self.wallet_id
    }

    /// Get secp256k1 public key address as hex
    pub fn address(&self) -> String {
        hex::encode(self.public_key.serialize())
    }

    /// Get public key bytes
    pub fn public_bytes(&self) -> Vec<u8> {
        self.public_key.serialize().to_vec()
    }

    /// Get secret key bytes (use with caution)
    pub fn secret_bytes(&self) -> Vec<u8> {
        self.secret_key.secret_bytes().to_vec()
    }

    /// Sign a message
    pub fn sign(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let hash = Sha256::digest(data);
        let message = Message::from_digest_slice(&hash)
            .context("Failed to create message from hash")?;
        let signature = self.secp.sign_ecdsa(message, &self.secret_key);

        Ok(signature.serialize_compact().to_vec())
    }

    /// Create a signed request for any serializable data
    pub fn sign_request<T: Serialize>(&self, data: T) -> Result<SignedRequest<T>> {
        SignedRequest::new(self, data)
    }

    /// Export public information as JSON
    pub fn public_info(&self) -> serde_json::Value {
        serde_json::json!({
            "wallet_id": self.id(),
            "public_key": hex::encode(self.public_bytes()),
        })
    }

    /// Display wallet info to console
    pub fn display(&self) {
        println!("\n🔐 Worker Identity:");
        println!("{}", serde_json::to_string_pretty(&self.public_info()).unwrap());
    }


}