use secp256k1::rand::{rng, Rng};
use serde::{Deserialize, Serialize};
use sodalite::{secretbox, secretbox_open, SecretboxKey, SecretboxNonce, SECRETBOX_KEY_LEN, SECRETBOX_NONCE_LEN};
use crate::wallet::error::WalletError;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use crate::wallet::secp_wallet::Wallet;

pub const SECRETBOX_BOXZEROBYTES: usize = 16;
pub const SECRETBOX_ZEROBYTES: usize = 32;

pub const PKCS8_DIVIDER: [u8; 5] = [161, 35, 3, 33, 0];
pub const PKCS8_HEADER: [u8; 16] = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32];

pub const SEC_LENGTH: usize = 64;
pub const SEED_LENGTH: usize = 32;

pub fn encode(
    secret_key: &[u8],
    public_key: &[u8],
    passphrase: Option<String>,
) -> Result<Vec<u8>, WalletError> {
    let sec_length: usize = secret_key.len();
    let pub_length: usize = public_key.len();

    let encoded_length: usize = PKCS8_HEADER.len() + sec_length + PKCS8_DIVIDER.len() + pub_length;
    let mut encoded = vec![0u8; encoded_length];

    let end = PKCS8_HEADER.len();
    encoded[..end].copy_from_slice(&PKCS8_HEADER[..]);

    let start = PKCS8_HEADER.len();
    let end = start + sec_length;
    encoded[start..end].copy_from_slice(secret_key);

    let start = PKCS8_HEADER.len() + sec_length;
    let end = start + PKCS8_DIVIDER.len();
    encoded[start..end].copy_from_slice(&PKCS8_DIVIDER[..]);

    let start = PKCS8_HEADER.len() + sec_length + PKCS8_DIVIDER.len();
    encoded[start..].copy_from_slice(public_key);

    let passphrase: String = match passphrase {
        Some(v) if !v.is_empty() => v,
        _ => return Ok(encoded),
    };

    let pass_bytes = passphrase.as_bytes();

    let mut key = [0u8; SECRETBOX_KEY_LEN];
    key[..pass_bytes.len()].copy_from_slice(pass_bytes);

    let mut rng = rng();
    let mut nonce = [0u8; SECRETBOX_NONCE_LEN];
    rng.fill(&mut nonce);

    let mut msg = vec![0u8; SECRETBOX_ZEROBYTES + encoded_length];
    msg[SECRETBOX_ZEROBYTES..].copy_from_slice(&encoded[..]);

    let mut encrypted = vec![0u8; msg.len()];
    secretbox(&mut encrypted, &msg, &nonce, &key).map_err(|_| ()).expect("secret failed");

    let result_length: usize = encoded_length + SECRETBOX_NONCE_LEN + SECRETBOX_BOXZEROBYTES;
    let mut result = vec![0u8; result_length];

    result[..SECRETBOX_NONCE_LEN].copy_from_slice(&nonce[..]);
    result[SECRETBOX_NONCE_LEN..].copy_from_slice(&encrypted[SECRETBOX_BOXZEROBYTES..]);

    Ok(result)
}

pub fn decode(encoded: &[u8], passphrase: Option<String>) -> Result<(Vec<u8>, Vec<u8>), WalletError> {
    let encoded_length = encoded.len();

    let mut nonce: SecretboxNonce = [0u8; SECRETBOX_NONCE_LEN];
    nonce.copy_from_slice(&encoded[0..SECRETBOX_NONCE_LEN]);

    let msg = match passphrase {
        Some(passphrase) if !passphrase.is_empty() => {
            let pass_bytes = passphrase.as_bytes();
            let mut key: SecretboxKey = [0u8; SECRETBOX_KEY_LEN];
            key[..pass_bytes.len()].copy_from_slice(pass_bytes);

            let mut encrypted =
                vec![0u8; SECRETBOX_BOXZEROBYTES + encoded_length - SECRETBOX_NONCE_LEN];
            encrypted[SECRETBOX_BOXZEROBYTES..].copy_from_slice(&encoded[SECRETBOX_NONCE_LEN..]);

            let mut raw = vec![0u8; encrypted.len()];
            match secretbox_open(&mut raw, &encrypted, &nonce, &key){
                Ok(_) => {},
                Err(_) => {
                    println!("❌ Wrong passphrase or corrupted keystore");
                    return Err(WalletError::ParseKeystoreError)
                },
            };

            let mut decrypted = vec![0u8; raw.len() - SECRETBOX_ZEROBYTES];
            decrypted.copy_from_slice(&raw[SECRETBOX_ZEROBYTES..]);
            decrypted
        },
        _ => encoded.to_vec(),
    };

    let mut header = [0u8; PKCS8_HEADER.len()];
    header.copy_from_slice(&msg[..PKCS8_HEADER.len()]);

    if header != PKCS8_HEADER {
        return Err(WalletError::ParseKeystoreError);
    }

    let mut secret_key = [0u8; SEC_LENGTH];
    let start: usize = PKCS8_HEADER.len();
    let end: usize = PKCS8_HEADER.len() + SEC_LENGTH;
    secret_key.copy_from_slice(&msg[start..end]);

    let divider_offset = PKCS8_HEADER.len() + SEC_LENGTH;
    let divider_end = divider_offset + PKCS8_DIVIDER.len();
    let mut divider = [0u8; PKCS8_DIVIDER.len()];
    divider.copy_from_slice(&msg[divider_offset..divider_end]);

    if divider != PKCS8_DIVIDER {
        let mut secret_key = [0u8; SEED_LENGTH];
        let start: usize = PKCS8_HEADER.len();
        let end: usize = PKCS8_HEADER.len() + SEED_LENGTH;
        secret_key.copy_from_slice(&msg[start..end]);

        let divider_offset = PKCS8_HEADER.len() + secret_key.len();
        let divider_end = divider_offset + PKCS8_DIVIDER.len();
        let mut divider = [0u8; PKCS8_DIVIDER.len()];
        divider.copy_from_slice(&msg[divider_offset..divider_end]);

        if divider != PKCS8_DIVIDER {
            return Err(WalletError::ParseKeystoreError);
        }

        let pub_offset = PKCS8_HEADER.len() + secret_key.len() + PKCS8_DIVIDER.len();
        let mut public_key: Vec<u8> = vec![0u8; msg.len() - pub_offset];
        public_key.copy_from_slice(&msg[pub_offset..]);

        Ok((public_key.to_vec(), secret_key.to_vec()))
    } else {
        let pub_offset = PKCS8_HEADER.len() + secret_key.len() + PKCS8_DIVIDER.len();
        let mut public_key = vec![0u8; msg.len() - pub_offset];
        public_key.copy_from_slice(&msg[pub_offset..]);

        Ok((public_key.to_vec(), secret_key.to_vec()))
    }
}


/// Keystore file format
#[derive(Debug, Serialize, Deserialize)]
pub struct KeystoreFile {
    pub wallet_id: String,
    pub secp_address: String,
    pub encoded: String,
    #[serde(default)]
    pub encrypted: bool,
}

impl Wallet {
    /// Save wallet to file with optional password encryption
    pub fn save(&self, path: &Path, password: Option<&str>) -> Result<()> {
        let encoded = encode(
            &self.secret_bytes(),
            &self.public_bytes(),
            password.map(|s| s.to_string())
        ).context("Failed to encode wallet data")?;

        let keystore = KeystoreFile {
            wallet_id: self.id(),
            secp_address: self.address(),
            encoded: format!("0x{}", hex::encode(encoded)),
            encrypted: password.is_some(),
        };

        let json = serde_json::to_string_pretty(&keystore)
            .context("Failed to serialize keystore")?;

        fs::write(path, json)
            .context("Failed to write keystore file")?;

        tracing::info!("Wallet saved: {}", self.id());
        Ok(())
    }

    /// Load wallet from file
    pub fn load(path: &Path, password: Option<&str>) -> Result<Self> {
        let data = fs::read_to_string(path)
            .context("Failed to read keystore file")?;

        let keystore: KeystoreFile = serde_json::from_str(&data)
            .context("Failed to parse keystore file")?;

        let encoded = keystore.encoded.strip_prefix("0x")
            .unwrap_or(&keystore.encoded);

        let encoded_bytes = hex::decode(encoded)
            .context("Failed to decode hex data")?;

        let (public_bytes, secret_bytes) = decode(
            &encoded_bytes,
            password.map(|s| s.to_string())
        ).context("Failed to decrypt wallet data")?;

        let wallet = Self::from_bytes(&secret_bytes)?;

        // Verify integrity
        if wallet.id() != keystore.wallet_id {
            anyhow::bail!("Wallet ID mismatch - file may be corrupted");
        }
        if wallet.public_bytes() != public_bytes {
            anyhow::bail!("Public key mismatch - file may be corrupted");
        }

        Ok(wallet)
    }
}