use anyhow::Result;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::poseidon::PoseidonPermutation, plonk::config::PoseidonGoldilocksConfig};
use psy_common::{args::SignType, data::qhashout::QHashOut};
use psy_common_circuit::circuits::zk_signature3::manager::SimplePsyZKSignatureManager;
use psy_crypto::{
    hash::traits::qhashable::QFieldHashable,
    signature::{
        secp256k1::wallet::{get_secp_public_key, hash_no_pad_compressed_public_key, CompressedPublicKeyToP2PKH, MemorySecp256K1Wallet},
        zk::wallet::SimplePsyPrivateKey,
    },
};
use psy_data::config::store_config::PsyHasher;
use psy_prover::wallet::memory_wallet::{get_secp256k1_fingerprint, get_zk_fingerprint};
use psy_rust_sdk::wallet::secp_wallet::Wallet;
use serde::{Deserialize, Serialize};

use super::args::RandomWalletArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

#[derive(Serialize, Deserialize)]
struct RandomWalletOutputJSON {
    private_key: QHashOut<GoldilocksField>,
    public_key: QHashOut<GoldilocksField>,
    public_key_param: QHashOut<GoldilocksField>,
    fingerprint: QHashOut<GoldilocksField>,
    sign_type: String,
}

pub fn run(args: RandomWalletArgs) -> Result<()> {
    let private_key = QHashOut::<GoldilocksField>::rand();

    let (fingerprint, public_key_param, public_key, sign_type_str) = match args.sign_type {
        SignType::ZKSign => {
            let fingerprint = get_zk_fingerprint();
            let private_key_obj = SimplePsyPrivateKey { private_key };
            let public_key_param = private_key_obj.get_public_key_param::<PsyHasher>();
            let pk_info = psy_crypto::signature::zk::data::ZKPublicKeyInfo {
                fingerprint,
                public_key_param,
            };
            let public_key = pk_info.qfhash::<PsyHasher>();
            (fingerprint, public_key_param, public_key, "ZK".to_string())
        }
        SignType::SECP256K1Sign => {
            let fingerprint = get_secp256k1_fingerprint();
            let secp_public_key = get_secp_public_key(private_key)?;
            let public_key_param = hash_no_pad_compressed_public_key::<GoldilocksField, PoseidonPermutation<GoldilocksField>>(secp_public_key);
            let pk_info = psy_crypto::signature::zk::data::ZKPublicKeyInfo {
                fingerprint,
                public_key_param,
            };
            let public_key = pk_info.qfhash::<PsyHasher>();
            (fingerprint, public_key_param, public_key, "SECP256K1".to_string())
        }
        _ => {
            anyhow::bail!("Unsupported sign type: {:?}", args.sign_type);
        }
    };

    let output = RandomWalletOutputJSON {
        private_key,
        public_key,
        public_key_param,
        fingerprint,
        sign_type: sign_type_str,
    };

    println!("Generated wallet:");
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
