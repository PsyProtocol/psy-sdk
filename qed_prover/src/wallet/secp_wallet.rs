use alloy_primitives::{Address, B256};
use alloy_signer::{Signer, SignerSync};
use alloy_signer_local::{PrivateKeySigner, MnemonicBuilder, coins_bip39::{English, Mnemonic}};
use anyhow::{Context, Result, bail};
use k256::{ecdsa::SigningKey, sha2::{Digest, Sha256}};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::PoseidonPermutation;
use qed_core::data::{qhashout::QHashOut, secp256k1::CompressedPublicKey};
use serde::Serialize;
use std::{fs, path::Path, fmt};

use crate::wallet::secp_sign::SignedRequest;

/// Configuration for wallet operations
pub struct WalletConfig {
    pub default_keystore_dir: Option<String>,
    pub default_password: Option<String>,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            default_keystore_dir: dirs::home_dir()
                .map(|h| h.join(".Psy/keystores").to_string_lossy().into_owned()),
            default_password: None,
        }
    }
}
/// Wrapper around Alloy's LocalWallet with your custom functionality
#[derive(Clone)]
pub struct Wallet {
    inner: PrivateKeySigner,
    wallet_id: QHashOut<GoldilocksField>,
}
impl Wallet {
    /// Create a new random wallet
    pub fn new() -> Result<Self> {
        Self::random()
    }
    /// Create a random wallet (explicit method)
    pub fn random() -> Result<Self> {
        let mut rng = rand::thread_rng();
        let inner = PrivateKeySigner::random_with(&mut rng);
        Self::from_signer(inner)
    }
    /// Create wallet from existing secret key bytes (32 bytes)
    pub fn from_bytes(key: &[u8]) -> Result<Self> {
        ensure_key_length(key)?;

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(key);

        let signing_key = SigningKey::from_bytes(&key_array.into())
            .context("Invalid private key")?;

        Self::from_signer(PrivateKeySigner::from_signing_key(signing_key))
    }

    /// Create wallet from hex private key string (matching Foundry's pattern)
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = parse_hex(hex)?;
        Self::from_bytes(&bytes)
    }

    /// Create wallet from mnemonic phrase
    pub fn from_mnemonic(phrase: &str, index: u32) -> Result<Self> {
        let derivation_path = format!("m/44'/60'/0'/0/{}", index);

        let wallet = MnemonicBuilder::<English>::default()
            .phrase(phrase)
            .derivation_path(derivation_path)?
            .build()?;

        Self::from_signer(wallet)
    }

    /// Internal constructor from signer
    fn from_signer(inner: PrivateKeySigner) -> Result<Self> {
        let compressed = compress_public_key(&inner.public_key().as_slice())?;
        let wallet_id = compute_wallet_id(compressed);

        Ok(Self { inner, wallet_id })
    }

    /// Generate a new mnemonic phrase
    pub fn generate_mnemonic(word_count: usize) -> Result<String> {
        validate_word_count(word_count)?;

        let mut rng = rand::thread_rng();
        let mnemonic = Mnemonic::<English>::new_with_count(&mut rng, word_count)
            .context("Failed to generate mnemonic")?;

        Ok(mnemonic.to_phrase())
    }
    /// Generate a vanity address with optional prefix/suffix
    pub fn vanity(prefix: Option<&str>, suffix: Option<&str>) -> Result<Self> {
        use rayon::prelude::*;

        let matcher = create_vanity_matcher(prefix, suffix);

        let wallet = (0..u64::MAX)
            .into_par_iter()
            .map(|_| {
                let mut rng = rand::thread_rng();
                PrivateKeySigner::random_with(&mut rng)
            })
            .find_any(|w| matcher(&w.address()))
            .ok_or_else(|| anyhow::anyhow!("Vanity address generation failed"))?;

        Self::from_signer(wallet)
    }

    /// Get wallet ID as string
    pub fn id(&self) -> String {
        self.wallet_id.to_string()
    }

    /// Get wallet ID as QHashOut
    pub fn id_hash(&self) -> QHashOut<GoldilocksField> {
        self.wallet_id
    }

    /// Get checksummed Ethereum address
    pub fn address(&self) -> String {
        self.inner.address().to_checksum(None)
    }

    /// Get raw address
    pub fn address_raw(&self) -> Address {
        self.inner.address()
    }

    /// Get public key bytes
    pub fn public_bytes(&self) -> Vec<u8> {
        self.inner.public_key().to_vec()
    }

    /// Get private key bytes (32 bytes)
    pub fn private_key(&self) -> Vec<u8> {
        self.inner.credential().to_bytes().to_vec()
    }

    /// Get private key as hex string
    pub fn private_key_hex(&self) -> String {
        format!("0x{}", hex::encode(self.private_key()))
    }

    /// Sign message with Ethereum personal_sign format
    pub fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>> {
        self.inner
            .sign_message_sync(message)
            .map(|sig| sig.as_bytes().to_vec())
            .context("Failed to sign message")
    }

    /// Sign raw data (SHA-256 hash, no Ethereum prefix)
    pub fn sign_raw(&self, data: &[u8]) -> Result<Vec<u8>> {
        let hash = Sha256::digest(data);

        self.inner
            .sign_hash_sync(&B256::from(hash.as_ref()))
            .map(|sig| sig.as_bytes().to_vec())
            .context("Failed to sign hash")
    }
    /// Create a signed request for any serializable data
    pub fn sign_request<T: Serialize>(&self, data: T) -> Result<SignedRequest<T>> {
        SignedRequest::new(self, data)
    }

    /// Verify signature against address
    pub fn verify_signature(message: &[u8], signature: &[u8], address: Address) -> Result<bool> {
        use alloy_primitives::Signature;

        let sig = Signature::try_from(signature)?;
        let recovered = sig.recover_address_from_msg(message)?;

        Ok(recovered == address)
    }
    /// Save wallet using Foundry's encrypt_keystore method
    pub fn save(&self, path: &Path, password: Option<&str>) -> Result<()> {
        // Ensure the directory exists
        ensure_parent_dir(path)?;

        let password = password.unwrap_or("");

        // Use PrivateKeySigner's encrypt_keystore method (as seen in Foundry)
        let mut rng = rand::thread_rng();
        let private_key = self.inner.credential().to_bytes();

        // If path is a directory, generate a filename
        let (dir, name) = split_path(path);

        let (_, uuid) = PrivateKeySigner::encrypt_keystore(
            dir.clone(),
            &mut rng,
            private_key,
            password,
            name.as_deref(),
        ).context("Failed to encrypt keystore")?;

        let final_path = build_keystore_path(&dir, name.as_deref(), &uuid);

        tracing::info!("Wallet saved to: {}", final_path.display());
        Ok(())
    }

    /// Load wallet from keystore file
    pub fn load(path: &Path, password: Option<&str>) -> Result<Self> {
        let password = password.unwrap_or("");
        let inner = PrivateKeySigner::decrypt_keystore(path, password)?;
        Self::from_signer(inner)
    }
    /// Create new keystore with optional name
    pub fn new_keystore(
        dir: &Path,
        password: &str,
        name: Option<&str>,
    ) -> Result<(Self, String)> {
        let mut rng = rand::thread_rng();
        let (inner, uuid) = PrivateKeySigner::new_keystore(dir, &mut rng, password, name)?;

        let wallet = Self::from_signer(inner)?;
        Ok((wallet, uuid))
    }

    /// Export to JSON keystore format
    pub fn to_json(&self, password: Option<&str>) -> Result<String> {
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("keystore");

        self.save(&temp_path, password)?;
        fs::read_to_string(&temp_path).context("Failed to read keystore JSON")
    }

    /// Import from JSON keystore string
    pub fn from_json(json: &str, password: Option<&str>) -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("keystore");

        fs::write(&temp_path, json)?;
        Self::load(&temp_path, password)
    }

    /// List all accounts in keystore directory
    pub fn list_accounts(dir: Option<&Path>) -> Result<Vec<String>> {
        let keystore_dir = resolve_keystore_dir(dir)?;

        if !keystore_dir.exists() {
            return Ok(Vec::new());
        }

        let accounts = fs::read_dir(keystore_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .filter_map(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from)
            })
            .collect();

        Ok(accounts)
    }
    /// Get the underlying PrivateKeySigner for advanced operations
    pub fn signer(&self) -> &PrivateKeySigner {
        &self.inner
    }
    /// Get public information as JSON
    pub fn info(&self) -> serde_json::Value {
        serde_json::json!({
            "wallet_id": self.id(),
            "address": self.address(),
            "public_key": hex::encode(self.public_bytes()),
        })
    }
    pub fn display(&self) -> serde_json::Value {
        serde_json::json!({
            "wallet_id": self.id(),
            "address": self.address(),
            "public_key": hex::encode(self.public_bytes()),
            "private_key": self.private_key_hex(),
        })
    }

}

impl fmt::Display for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Wallet[{}]", &self.address()[..10])
    }
}

impl fmt::Debug for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("id", &self.id())
            .field("address", &self.address())
            .finish()
    }
}

fn ensure_key_length(key: &[u8]) -> Result<()> {
    if key.len() != 32 {
        bail!("Private key must be exactly 32 bytes, got {}", key.len());
    }
    Ok(())
}

fn parse_hex(hex: &str) -> Result<Vec<u8>> {
    let clean = hex.strip_prefix("0x").unwrap_or(hex);
    hex::decode(clean).context("Invalid hex string")
}

fn validate_word_count(count: usize) -> Result<()> {
    const VALID_COUNTS: &[usize] = &[12, 15, 18, 21, 24];
    if !VALID_COUNTS.contains(&count) {
        bail!("Invalid word count. Must be one of: {:?}", VALID_COUNTS);
    }
    Ok(())
}

fn compress_public_key(public_key: &[u8]) -> Result<CompressedPublicKey> {
    match public_key.len() {
        65 => {
            let mut compressed = [0u8; 33];
            compressed[0] = if public_key[64] & 1 == 0 { 0x02 } else { 0x03 };
            compressed[1..].copy_from_slice(&public_key[1..33]);
            Ok(CompressedPublicKey(compressed))
        }
        64 => {
            let mut compressed = [0u8; 33];
            compressed[0] = if public_key[63] & 1 == 0 { 0x02 } else { 0x03 };
            compressed[1..].copy_from_slice(&public_key[0..32]);
            Ok(CompressedPublicKey(compressed))
        }
        _ => bail!("Invalid public key length: {}", public_key.len()),
    }
}

fn compute_wallet_id(compressed: CompressedPublicKey) -> QHashOut<GoldilocksField> {
    crate::wallet::utils::hash_no_pad_compressed_public_key::<
        GoldilocksField,
        PoseidonPermutation<GoldilocksField>
    >(compressed)
}

fn create_vanity_matcher(prefix: Option<&str>, suffix: Option<&str>) -> impl Fn(&Address) -> bool {
    let prefix = prefix.map(String::from);
    let suffix = suffix.map(String::from);

    move |addr: &Address| {
        let hex = hex::encode(addr.as_slice());
        prefix.as_ref().map_or(true, |p| hex.starts_with(p)) &&
            suffix.as_ref().map_or(true, |s| hex.ends_with(s))
    }
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn split_path(path: &Path) -> (std::path::PathBuf, Option<String>) {
    if path.is_dir() {
        (path.to_path_buf(), None)
    } else {
        (
            path.parent().unwrap_or(Path::new(".")).to_path_buf(),
            path.file_name().and_then(|n| n.to_str()).map(String::from),
        )
    }
}

fn build_keystore_path(dir: &Path, name: Option<&str>, uuid: &str) -> std::path::PathBuf {
    name.map_or_else(
        || dir.join(uuid),
        |n| dir.join(n),
    )
}

fn resolve_keystore_dir(dir: Option<&Path>) -> Result<std::path::PathBuf> {
    dir.map(|p| p.to_path_buf())
        .or_else(|| dirs::home_dir().map(|h| h.join(".foundry/keystores")))
        .ok_or_else(|| anyhow::anyhow!("Could not determine keystore directory"))
}