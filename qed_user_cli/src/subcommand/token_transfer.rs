use std::str::FromStr;

use anyhow::Result;

use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;

use crate::{constant::get_network_magic_for_str, rpc::provider::RpcProvider};

use super::args::TokenTransferArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;

pub async fn run(args: TokenTransferArgs) -> Result<()> {
    let provider = RpcProvider::new(&args.rpc_config_path)?;

    let network_magic = get_network_magic_for_str(args.network)?;

    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();

    let public_key: QHashOut<GoldilocksField> = wallet.add_private_key(SimpleQEDPrivateKey {
        private_key: private_key,
    });

    // let city_token_transfer_rpcrequest = wallet.sign_l2_transfer(
    //     public_key,
    //     network_magic,
    //     args.from,
    //     args.to,
    //     args.value,
    //     args.nonce,
    // )?;

    // provider
    //     .token_transfer::<F>(city_token_transfer_rpcrequest)
    //     .await?;

    // Ok(())

    unimplemented!()
}
