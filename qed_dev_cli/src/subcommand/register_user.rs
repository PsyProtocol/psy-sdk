use core::time;
use std::str::FromStr;

use anyhow::Ok;
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::{
    hash::traits::qhashable::QFieldHashable,
    signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey},
};
use qed_data::config::store_config::QEDHasher;
use serde::{Deserialize, Serialize};

use qed_prover::local::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QRegisterUserRPCRequest,
};

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub private_key: QHashOut<GoldilocksField>,
    pub public_key: ZKPublicKeyInfo<GoldilocksField>,
}

pub async fn run(args: super::RegisterUserArgs) -> anyhow::Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    if args.private_key.is_empty() {
        anyhow::bail!("you must provide --private-key");
    }

    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let public_key = wallet.add_private_key_get_info(SimpleQEDPrivateKey::new(private_key));

    println!("{}", serde_json::to_string_pretty(&public_key)?);
    println!("{:?}", public_key.qfhash::<QEDHasher>().to_string());
    provider.register_user(QRegisterUserRPCRequest {
        public_key: public_key,
    }).await?;

    println!("{}", serde_json::to_string_pretty(&public_key).unwrap());

    Ok(())
}

pub async fn run_random(args: super::RandomArgs) -> anyhow::Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;

    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();

    for i in 0..args.total_user {
        let private_key = QHashOut::<GoldilocksField>::rand();
        let public_key = wallet.add_private_key_get_info(SimpleQEDPrivateKey::new(private_key));

        provider.register_user(QRegisterUserRPCRequest {
            public_key: public_key,
        }).await?;

        let keypair = KeyPair {
            public_key,
            private_key,
        };

        tracing::info!("user {}: {}", i, serde_json::to_string_pretty(&keypair)?,);
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(time::Duration::from_millis(100));

        if i % args.user_per_block == 0 && i != 0 {
            provider.produce_block::<GoldilocksField>().await?;
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::sleep(time::Duration::from_secs(args.interval));
        }
    }

    if args.total_user % args.user_per_block != 0 {
        provider.produce_block::<GoldilocksField>().await?;
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(time::Duration::from_secs(args.interval));
    }

    Ok(())
}