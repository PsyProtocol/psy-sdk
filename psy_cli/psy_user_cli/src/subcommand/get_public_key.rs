use std::str::FromStr;

use kvq::traits::KVQSerializable;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    hash::poseidon::{PoseidonHash, PoseidonPermutation},
    plonk::config::PoseidonGoldilocksConfig,
};
use psy_common::{
    args::SignType,
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
};
use psy_common_circuit::circuits::zk_signature3::manager::SimplePsyZKSignatureManager;
use psy_config::PSY_NETWORK_MAGIC;
use psy_crypto::{
    hash::traits::qhashable::QFieldHashable,
    signature::{
        secp256k1::wallet::{get_secp_public_key, hash_no_pad_compressed_public_key, CompressedPublicKeyToP2PKH, MemorySecp256K1Wallet},
        zk::{
            data::ZKPublicKeyInfo,
            wallet::{SimplePrivateKey, SimplePsyPrivateKey},
        },
    },
};
use psy_data::config::store_config::PsyHasher;
use psy_prover::wallet::memory_wallet::{get_secp256k1_fingerprint, get_zk_fingerprint, PsyMemoryWallet};
use psy_rust_sdk::wallet::secp_wallet::Wallet;
use psy_ups_circuit::circuit_manager::core::PsyUPSStepCircuitManager;
use psy_vm::ups::circuit_manager::UPSCircuitManager;

use super::args::GetPublicKeyArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

pub async fn run(args: GetPublicKeyArgs) -> anyhow::Result<()> {
    let private_key_base = QHashOut::<GoldilocksField>::from_str(&args.private_key)?;

    let (fingerprint, public_key_param, public_key, sign_type_str) = match args.sign_type {
        SignType::ZKSign => {
            let fingerprint = get_zk_fingerprint();
            let private_key_obj = SimplePsyPrivateKey {
                private_key: private_key_base,
            };
            let public_key_param = private_key_obj.get_public_key_param::<PsyHasher>();
            let pk_info = ZKPublicKeyInfo {
                fingerprint,
                public_key_param,
            };
            let public_key = pk_info.qfhash::<PsyHasher>();
            (fingerprint, public_key_param, public_key, "ZK".to_string())
        }
        SignType::SECP256K1Sign => {
            let fingerprint = get_secp256k1_fingerprint();
            let secp_public_key = get_secp_public_key(private_key_base)?;
            let public_key_param = hash_no_pad_compressed_public_key::<GoldilocksField, PoseidonPermutation<GoldilocksField>>(secp_public_key);
            let pk_info = ZKPublicKeyInfo {
                fingerprint,
                public_key_param,
            };
            let public_key = pk_info.qfhash::<PsyHasher>();
            (fingerprint, public_key_param, public_key, "SECP256K1".to_string())
        }
        SignType::SoftwareDefinedDPNSign | SignType::SoftwareDefinedPlonky2Sign => {
            anyhow::bail!("Software Defined signatures require circuit setup - use a different tool");
        }
    };

    println!("Public Key Info ({})", sign_type_str);
    println!("  public_key_param: {}", public_key_param);
    println!("  fingerprint: {}", fingerprint);
    println!("  public_key: {}", public_key);

    Ok(())
}
