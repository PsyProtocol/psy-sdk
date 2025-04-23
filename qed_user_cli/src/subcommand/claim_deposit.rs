use std::str::FromStr;

use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};

use anyhow::Result;
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::data::{base_types::hash256::Hash256, qhashout::QHashOut};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;

use crate::{constant::get_network_magic_for_str, rpc::provider::RpcProvider};

use super::args::ClaimDepositArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;

pub async fn run(args: ClaimDepositArgs) -> Result<()> {
    let provider = RpcProvider::new(&args.rpc_config_path)?;

    let network_magic = get_network_magic_for_str(args.network)?;

    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let private_key = QHashOut::<F>::from_str(&args.private_key)?;

    wallet.add_private_key(SimpleQEDPrivateKey { private_key });

    let txid = Hash256::from_hex_string(&args.txid)?;

    // let deposit = provider.get_deposit_by_txid(txid).await?;

    // let city_claim_deposit_request =
    //     wallet.sign_claim_deposit(network_magic, args.user_id, &deposit.to_city_l1_deposit())?;

    // provider
    //     .claim_deposit::<F>(city_claim_deposit_request)
    //     .await?;

    // Ok(())

    unimplemented!()
}
