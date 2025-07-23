use crate::subcommand::args::{KeyType, RegisterUserArgs};
use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::PoseidonPermutation;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_crypto::signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey};
use qed_data::config::store_config::QEDHasher;
use qed_prover::local::provider::{QUserRpcProvider, RpcProvider};
use qed_prover::local::request::QRegisterUserRPCRequest;
use qed_prover::wallet::utils::{get_secp_public_key, hash_no_pad_compressed_public_key};
use std::str::FromStr;

pub fn run(args: RegisterUserArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;

    // Parse private key
    let private_key_base = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("Failed to parse private key: {}", e))?;
    // Get public key info
    let public_key_param = match KeyType::from(args.key_type.clone()) {
        KeyType::ZK => SimpleQEDPrivateKey {
            private_key: private_key_base,
        }
        .get_public_key_param::<QEDHasher>(),
        KeyType::SECP256K1 => {
            let compressed_public_key = get_secp_public_key(private_key_base)?;
            tracing::info!("compressed public key {:?}", compressed_public_key);
            hash_no_pad_compressed_public_key::<GoldilocksField, PoseidonPermutation<GoldilocksField>>(
                compressed_public_key,
            )
        }
        KeyType::SoftwareDefined => unimplemented!(),
    };
    let fingerprint = args
        .fingerprint
        .map(|f| QHashOut::<GoldilocksField>::from_str(&f))
        .transpose()?
        .unwrap_or_else(|| {
            // Default fingerprint - this should match the circuit fingerprint
            match KeyType::from(args.key_type) {
                KeyType::ZK => QHashOut::<GoldilocksField>::from_string_or_panic(
                    "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0",
                ),
                KeyType::SECP256K1 => QHashOut::<GoldilocksField>::from_string_or_panic(
                    "795c781a246206d4d1efc7cf566c31319928c52957efc5cb4f27362d94a4976f",
                ),
                KeyType::SoftwareDefined => unimplemented!(),
            }
        });

    let public_key_info = ZKPublicKeyInfo {
        fingerprint,
        public_key_param,
    };

    // Register user
    provider.register_user(QRegisterUserRPCRequest {
        public_key: public_key_info,
    })?;

    // Output the result
    let public_key_hash = public_key_info.qfhash::<QEDHasher>();
    println!("{{");
    println!("  \"public_key_hash\": \"{}\",", public_key_hash);
    println!("  \"fingerprint\": \"{}\",", public_key_info.fingerprint);
    println!(
        "  \"public_key_param\": \"{}\"",
        public_key_info.public_key_param
    );
    println!("}}");

    Ok(())
}
