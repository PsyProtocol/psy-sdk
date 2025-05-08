use std::str::FromStr;

use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_common::link::data::BTCAddress160;
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::secp256k1::wallet::{CompressedPublicKeyToP2PKH, MemorySecp256K1Wallet};
use qed_crypto::signature::zk::wallet::{SimpleL2PrivateKey, SimpleQEDPrivateKey};
use qed_store::config::store_config::QEDHasher;

use super::args::GetPublicKeyArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

pub fn run(args: GetPublicKeyArgs) -> anyhow::Result<()> {
    let private_key_base = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let mut debug_wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let l1_public_key = MemorySecp256K1Wallet::new()
        .add_private_key(Hash256::from_hex_string(&args.private_key)?)?;
    let l2_public_key = debug_wallet.add_private_key_get_info(SimpleQEDPrivateKey {
        private_key: private_key_base,
    });

    println!("l1 public_key = {}", hex::encode(&l1_public_key.0));
    println!(
        "l1 p2pkh = {}",
        BTCAddress160::new_p2pkh(l1_public_key.to_p2pkh_address()).to_address_string()
    );
    println!(
        "public_key_param: {}, fingerprint: {}",
        l2_public_key.public_key_param.to_string(),
        l2_public_key.fingerprint.to_string()
    );
    println!("l2 public_key = {}", l2_public_key.to_hash::<QEDHasher>());
    Ok(())
}
