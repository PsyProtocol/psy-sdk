use secp256k1::{Secp256k1, SecretKey, PublicKey, Message};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use k256::sha2::{Digest, Sha256};
use crate::wallet::error::WalletError;
use crate::wallet::keystore::{decode, encode};
use secp256k1::rand::rng;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeystoreFile {
    pub address: String,
    pub encoded: String,
}
#[derive(Clone, Debug)]
pub struct Wallet {
    secret_key: SecretKey,
    public_key: PublicKey,
    secp: Secp256k1<secp256k1::All>,
}

impl Wallet {
    /// Create a new wallet with a random private key
    pub fn new() -> anyhow::Result<Self, WalletError> {
        let secp = Secp256k1::new();
        let mut rng = rng();
        let secret_key = SecretKey::new(&mut rng);
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);

        Ok(Wallet {
            secret_key,
            public_key,
            secp,
        })
    }

    /// Create wallet from existing secret key bytes
    pub fn from_secret_key_bytes(key_bytes: &[u8]) -> anyhow::Result<Self> {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(key_bytes)?;
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);

        Ok(Wallet {
            secret_key,
            public_key,
            secp,
        })
    }

    /// Get the public key as hex string (this serves as the worker ID)
    pub fn get_address(&self) -> String {
        hex::encode(self.public_key.serialize())
    }

    /// Get the public key bytes
    pub fn get_public_key_bytes(&self) -> Vec<u8> {
        self.public_key.serialize().to_vec()
    }

    /// Sign a message
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();

        let message = Message::from_digest_slice(&hash).expect("32 bytes");
        let signature = self.secp.sign_ecdsa(message, &self.secret_key);

        signature.serialize_compact().to_vec()
    }

    /// Save wallet to file with optional password encryption
    pub fn save_to_file(&self, path: &Path, password: Option<String>) -> anyhow::Result<(), WalletError> {
        let secret_bytes = self.secret_key.secret_bytes();
        let public_bytes = self.get_public_key_bytes();
        let encoded = encode(&secret_bytes, &public_bytes, password)?;

        let keystore = KeystoreFile {
            address: self.get_address(),
            encoded: format!("0x{}", hex::encode(encoded)),
        };

        let json = serde_json::to_string_pretty(&keystore)?;
        fs::write(path, json)?;

        Ok(())
    }

    /// Load wallet from file
    pub fn load_from_file(path: &Path, password: Option<String>) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)?;
        let keystore: KeystoreFile = serde_json::from_str(&data)?;

        let encoded = if keystore.encoded.starts_with("0x") {
            &keystore.encoded[2..]
        } else {
            &keystore.encoded
        };

        let encoded_bytes = hex::decode(encoded)?;
        let (pk, sk) = decode(&encoded_bytes, password)?;

        Self::from_secret_key_bytes(&sk)
    }
}
