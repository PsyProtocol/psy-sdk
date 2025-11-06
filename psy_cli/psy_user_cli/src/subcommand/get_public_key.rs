use std::str::FromStr;

use kvq::traits::KVQSerializable;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::poseidon::PoseidonHash, plonk::config::PoseidonGoldilocksConfig};
use psy_common::{
    args::SignType,
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
};
use psy_common_circuit::circuits::zk_signature3::manager::SimplePsyZKSignatureManager;
use psy_config::PSY_NETWORK_MAGIC;
use psy_crypto::{
    hash::traits::qhashable::QFieldHashable,
    signature::{
        secp256k1::wallet::{CompressedPublicKeyToP2PKH, MemorySecp256K1Wallet},
        zk::wallet::{SimplePrivateKey, SimplePsyPrivateKey},
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

    // Create wallet and circuit manager
    let main_circuits: Box<dyn UPSCircuitManager<C, D>> = Box::new(PsyUPSStepCircuitManager::<C, D>::new_with_config(
        psy_config::network_constants::PSY_NETWORK_MAGIC,
    ));
    let mut wallet = PsyMemoryWallet::new(vec![main_circuits]);

    // Get fingerprint based on sign type
    let fingerprint = match args.sign_type {
        SignType::ZKSign => get_zk_fingerprint(),
        SignType::SECP256K1Sign => get_secp256k1_fingerprint(),
        SignType::SoftwareDefinedDPNSign => {
            anyhow::bail!("Software Defined DPN requires fingerprint parameter");
        }
        SignType::SoftwareDefinedPlonky2Sign => wallet.register_plonky2_software_defined_circuit(32, 0).await?,
    };

    // Use simplified interface
    let public_key_info = wallet.get_or_create_user(private_key_base, fingerprint).await?;
    let public_key = public_key_info.qfhash::<PsyHasher>();

    println!("Public Key Info:");
    println!("  public_key_param: {}", public_key_info.public_key_param);
    println!("  fingerprint: {}", public_key_info.fingerprint);
    println!("  public_key_hash: {}", public_key);

    Ok(())
}
