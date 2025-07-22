use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey};
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_prover::local::provider::{QUserRpcProvider, RpcProvider};
use qed_prover::local::request::QRegisterUserRPCRequest;
use qed_data::config::store_config::QEDHasher;
use crate::subcommand::args::RegisterUserArgs;
use std::str::FromStr;

pub fn run(args: RegisterUserArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    
    // Parse private key
    let private_key_base = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("Failed to parse private key: {}", e))?;
    let private_key = SimpleQEDPrivateKey {
        private_key: private_key_base,
    };
    
    // Get public key info
    let public_key_param = private_key.get_public_key_param::<QEDHasher>();
    let fingerprint = args.fingerprint
        .map(|f| QHashOut::<GoldilocksField>::from_str(&f))
        .transpose()?
        .unwrap_or_else(|| {
            // Default fingerprint - this should match the circuit fingerprint
            QHashOut::<GoldilocksField>::from_string_or_panic("65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0")
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
    println!("  \"public_key_param\": \"{}\"", public_key_info.public_key_param);
    println!("}}");
    
    Ok(())
}