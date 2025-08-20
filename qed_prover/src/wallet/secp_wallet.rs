use alloy_primitives::{Address, B256};
use alloy_signer::{Signer, SignerSync};
use alloy_signer_local::PrivateKeySigner;
use anyhow::{Context, Result, bail};
use k256::{
    ecdsa::SigningKey,
    elliptic_curve::sec1::ToEncodedPoint,
};
use k256::sha2::{Digest, Sha256};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::PoseidonPermutation;
use qed_core::data::qhashout::QHashOut;
use qed_core::data::secp256k1::CompressedPublicKey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// For mnemonic support (matching Foundry's approach)
use alloy_signer_local::{
    MnemonicBuilder,
    coins_bip39::{English, Mnemonic},
};
use crate::wallet::secp_sign::SignedRequest;

/// Wrapper around Alloy's LocalWallet with your custom functionality
#[derive(Clone)]
pub struct Wallet {
    inner: PrivateKeySigner,
    wallet_id: QHashOut<GoldilocksField>,
}
impl Wallet {
    /// Create a new wallet with a random private key
    pub fn new() -> Result<Self> {
        let mut rng = rand::thread_rng();
        let inner = PrivateKeySigner::random_with(&mut rng);
        Self::from_private_key_signer(inner)
    }
    /// Create wallet from existing secret key bytes (32 bytes)
    pub fn from_bytes(key_bytes: &[u8]) -> Result<Self> {
        if key_bytes.len() != 32 {
            bail!("Private key must be exactly 32 bytes");
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(key_bytes);

        let signing_key = SigningKey::from_bytes(&bytes.into())
            .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;

        let inner = PrivateKeySigner::from_signing_key(signing_key);
        Self::from_private_key_signer(inner)
    }

    /// Create wallet from hex private key string (matching Foundry's pattern)
    pub fn from_hex(hex_key: &str) -> Result<Self> {
        let hex_str = hex_key.strip_prefix("0x").unwrap_or(hex_key);
        let bytes = hex::decode(hex_str)
            .context("Invalid hex string")?;
        Self::from_bytes(&bytes)
    }

    /// Create wallet from mnemonic phrase
    pub fn from_mnemonic(phrase: &str, index: u32) -> Result<Self> {
        let derivation_path = format!("m/44'/60'/0'/0/{}", index);

        let wallet = MnemonicBuilder::<English>::default()
            .phrase(phrase)
            .derivation_path(derivation_path)?
            .build()?;

        Self::from_private_key_signer(wallet)
    }

    /// Generate a new mnemonic phrase (following Foundry's approach)
    pub fn generate_mnemonic(words: usize) -> Result<String> {
        let mut rng = rand::thread_rng();
        let mnemonic = Mnemonic::<English>::new_with_count(&mut rng, words)
            .map_err(|e| anyhow::anyhow!("Failed to generate mnemonic: {:?}", e))?;
        Ok(mnemonic.to_phrase())
    }

    /// Internal constructor from PrivateKeySigner
    pub(crate) fn from_private_key_signer(inner: PrivateKeySigner) -> Result<Self> {
        // Get public key for wallet ID calculation
        let public_key = inner.public_key();

        // Create compressed public key for your custom ID
        let compressed = if public_key.len() == 65 {
            let mut compressed_bytes = [0u8; 33];
            compressed_bytes[0] = if public_key[64] & 1 == 0 { 0x02 } else { 0x03 };
            compressed_bytes[1..].copy_from_slice(&public_key[1..33]);
            CompressedPublicKey(compressed_bytes)
        } else if public_key.len() == 64 {
            // Uncompressed without prefix
            let mut compressed_bytes = [0u8; 33];
            compressed_bytes[0] = if public_key[63] & 1 == 0 { 0x02 } else { 0x03 };
            compressed_bytes[1..].copy_from_slice(&public_key[0..32]);
            CompressedPublicKey(compressed_bytes)
        } else {
            bail!("Invalid public key length: {}", public_key.len());
        };

        let wallet_id = crate::wallet::utils::hash_no_pad_compressed_public_key::<
            GoldilocksField,
            PoseidonPermutation<GoldilocksField>
        >(compressed);

        Ok(Self { inner, wallet_id })
    }

    /// Get wallet ID as string
    pub fn id(&self) -> String {
        self.wallet_id.to_string()
    }

    /// Get wallet ID as QHashOut
    pub fn id_hash(&self) -> QHashOut<GoldilocksField> {
        self.wallet_id
    }

    /// Get Ethereum address (checksummed, following Foundry's format)
    pub fn address(&self) -> String {
        self.inner.address().to_checksum(None)
    }

    /// Get public key bytes (following Foundry's public_key() method)
    pub fn public_bytes(&self) -> Vec<u8> {
        self.inner.public_key().to_vec()
    }

    /// Get secret key bytes (32 bytes, following Foundry's credential().to_bytes())
    pub fn secret_bytes(&self) -> Vec<u8> {
        self.inner.credential().to_bytes().to_vec()
    }

    /// Sign a message (Ethereum personal_sign format)
    pub fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>> {
        let signature = self.inner.sign_message_sync(message)
            .map_err(|e| anyhow::anyhow!("Failed to sign message: {}", e))?;

        // Convert to bytes format (65 bytes)
        Ok(signature.as_bytes().to_vec())
    }

    /// Sign raw data without Ethereum prefix (for your custom protocols)
    pub fn sign_raw(&self, data: &[u8]) -> Result<Vec<u8>> {
        let hash = Sha256::digest(data);
        let hash_bytes: [u8; 32] = hash.into();

        let signature = self.inner.sign_hash_sync(&B256::from(hash_bytes))
            .map_err(|e| anyhow::anyhow!("Failed to sign hash: {}", e))?;

        Ok(signature.as_bytes().to_vec())
    }
    /// Create a signed request for any serializable data
    pub fn sign_request<T: Serialize>(&self, data: T) -> Result<SignedRequest<T>> {
        SignedRequest::new(self, data)
    }
    /// Save wallet using Foundry's encrypt_keystore method
    pub fn save(&self, path: &Path, password: Option<&str>) -> Result<()> {
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let password = password.unwrap_or("");

        // Use PrivateKeySigner's encrypt_keystore method (as seen in Foundry)
        let mut rng = rand::thread_rng();
        let private_key = self.inner.credential().to_bytes();

        // If path is a directory, generate a filename
        let (dir, account_name) = if path.is_dir() {
            (path.to_path_buf(), None)
        } else {
            (path.parent().unwrap_or(Path::new(".")).to_path_buf(),
             path.file_name().and_then(|n| n.to_str()))
        };

        let (_, uuid) = PrivateKeySigner::encrypt_keystore(
            dir.clone(),
            &mut rng,
            private_key,
            password,
            account_name.clone(),
        ).context("Failed to encrypt keystore")?;

        let keystore_path = if account_name.is_some() {
            dir.join(account_name.unwrap())
        } else {
            dir.join(&uuid)
        };

        tracing::info!("Wallet saved to: {}", keystore_path.display());
        Ok(())
    }

    /// Load wallet from encrypted keystore file (following Foundry's decrypt_keystore)
    pub fn load(path: &Path, password: Option<&str>) -> Result<Self> {
        let password = password.unwrap_or("");
        let inner = PrivateKeySigner::decrypt_keystore(path, password)
            .context("Failed to decrypt keystore")?;

        Self::from_private_key_signer(inner)
    }
    /// Create a new keystore with specified name (following Foundry's new_keystore pattern)
    pub fn new_keystore(
        dir: &Path,
        password: &str,
        account_name: Option<&str>,
    ) -> Result<(Self, String)> {
        let mut rng = rand::thread_rng();
        let (inner, uuid) = PrivateKeySigner::new_keystore(
            dir,
            &mut rng,
            password,
            account_name,
        ).context("Failed to create new keystore")?;

        let wallet = Self::from_private_key_signer(inner)?;
        Ok((wallet, uuid))
    }

    /// Export to JSON keystore string (Web3 Secret Storage format)
    pub fn to_json(&self, password: Option<&str>) -> Result<String> {
        // Create a temporary directory for the export
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("temp_keystore");

        // Save to temp file
        self.save(&temp_path, password)?;

        // Read and return the JSON
        let json = fs::read_to_string(&temp_path)?;
        Ok(json)
    }

    /// Import from JSON keystore string
    pub fn from_json(json: &str, password: Option<&str>) -> Result<Self> {
        // Write to temporary file
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("temp_keystore");
        fs::write(&temp_path, json)?;

        // Load from temp file
        Self::load(&temp_path, password)
    }

    /// Get the underlying PrivateKeySigner for advanced operations
    pub fn signer(&self) -> &PrivateKeySigner {
        &self.inner
    }



    /// Verify signature (following Foundry's verify pattern)
    pub fn verify_signature(message: &[u8], signature: &[u8], address: Address) -> Result<bool> {
        use alloy_primitives::Signature;

        let sig = Signature::try_from(signature)
            .context("Invalid signature format")?;

        let recovered = sig.recover_address_from_msg(message)
            .context("Failed to recover address")?;

        Ok(recovered == address)
    }
    /// Export public information as JSON
    pub fn public_info(&self) -> serde_json::Value {
        serde_json::json!({
            "wallet_id": self.id(),
            "public_key": hex::encode(self.public_bytes()),
        })
    }
    /// Display wallet info (following Foundry's output format)
    pub fn display(&self) {
        println!("Successfully created new keypair.");
        println!("Wallet ID: {}", self.id());
        println!("Address:     {}", self.address());
        println!("Private key: 0x{}", hex::encode(self.secret_bytes()));
    }


}