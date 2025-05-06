use std::str::FromStr;

use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::{
    hash::traits::qhashable::QFieldHashable, signature::zk::wallet::SimpleQEDPrivateKey,
};
use qed_store::config::store_config::QEDHasher;

use crate::rpc::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QRegisterUserRPCRequest,
};

use super::args::RegisterUserArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

pub fn run(args: RegisterUserArgs) -> anyhow::Result<()> {
    let provider = RpcProvider::new_with_config(args.rpc_config)?;
    if args.private_key.is_empty() {
        anyhow::bail!("you must provide --private-key");
    }

    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let public_key = wallet.add_private_key_get_info(SimpleQEDPrivateKey::new(private_key));

    println!("{}", serde_json::to_string_pretty(&public_key).unwrap());
    println!("{:?}", public_key.qfhash::<QEDHasher>().to_string());
    provider.register_user(QRegisterUserRPCRequest {
        public_key: public_key,
    })?;

    println!("{}", serde_json::to_string_pretty(&public_key).unwrap());

    Ok(())
}
