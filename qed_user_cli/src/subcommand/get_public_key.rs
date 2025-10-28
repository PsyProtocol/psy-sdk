use std::str::FromStr;

use kvq::traits::KVQSerializable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use psy_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use psy_core::data::base_types::hash256::Hash256;
use psy_core::data::qhashout::QHashOut;
use psy_crypto::hash::traits::qhashable::QFieldHashable;
use psy_crypto::signature::secp256k1::wallet::{CompressedPublicKeyToP2PKH, MemorySecp256K1Wallet};
use psy_crypto::signature::zk::wallet::{SimpleL2PrivateKey, SimpleQEDPrivateKey};
use psy_data::config::store_config::QEDHasher;
use qed_prover::local::args::SignType;
use qed_prover::ups::circuit_manager::core::{QCircuitManager, QEDUPSStepCircuitManager};
use qed_prover::wallet::memory_wallet::QEDMemoryWallet;
use qed_prover::wallet::secp_wallet::Wallet;

use super::args::GetPublicKeyArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

pub async fn run(args: GetPublicKeyArgs) -> anyhow::Result<()> {
    match args.sign_type {
        SignType::ZKSign => {
            let private_key_base = QHashOut::<GoldilocksField>::from_str(&args.private_key)
                .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

            let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
            let public_key = wallet.add_private_key_get_info(SimpleQEDPrivateKey {
                private_key: private_key_base,
            });

            println!("ZK Signature Public Key:");
            println!(
                "  public_key_param: {}",
                public_key.public_key_param.to_string()
            );
            println!("  fingerprint: {}", public_key.fingerprint.to_string());
            println!("  public_key: {}", public_key.qfhash::<QEDHasher>());
        }
        SignType::SECP256K1Sign => {
            let wallet = Wallet::from_hex(&args.private_key)?;

            let main_circuits =
                QCircuitManager::Local(QEDUPSStepCircuitManager::<C, D>::new_with_config(
                    psy_core::config::network_constants::QED_NETWORK_MAGIC_REGTEST,
                ));

            let mut memory_wallet = QEDMemoryWallet::new(vec![Box::new(main_circuits)]);
            let private_key = QHashOut::from(Hash256::from_bytes(&wallet.private_key())?);
            let secp_pk_info = memory_wallet.add_secp_private_key(private_key).await?;
            let public_key = secp_pk_info.qfhash::<QEDHasher>();

            println!("Secp256k1 Signature Public Key:");
            println!("  ETH Address: {}", wallet.address());
            println!(
                "  Secp256k1 Public Key: {}",
                hex::encode(wallet.public_key())
            );
            println!(
                "  public_key_hash: {}",
                secp_pk_info.public_key_param.to_string()
            );
            println!("  fingerprint: {}", secp_pk_info.fingerprint.to_string());
            println!("  public_key: {}", public_key);
        }
        SignType::SoftwareDefinedSign => {
            println!("Software Defined signature type not supported for public key display");
        }
    }

    Ok(())
}
