use std::str::FromStr;

use anyhow::Result;

use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_common::link::data::BTCAddress160;
use qed_core::data::{base_types::hash160::Hash160, qhashout::QHashOut};

use crate::{
    constant::get_network_magic_for_str,
    rpc::provider::{QUserRpcProvider, RpcProvider},
};

use super::args::AddWithdrawalArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;

pub async fn run(args: AddWithdrawalArgs) -> Result<()> {
    // let destination = if args.destination.len() == 40 {
    //     Hash160::from_hex_string(&args.destination)?
    // } else {
    //     BTCAddress160::try_from_string(&args.destination)?.address
    // };
    // let provider = RpcProvider::new(&args.rpc_config_path)?;
    // let network_magic = get_network_magic_for_str(args.network)?;

    // let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
    //     .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    // let mut wallet = CityMemoryWallet::<C, D>::new_fast_setup();

    // let public_key = wallet.add_zk_private_key(private_key);

    // let req = wallet.sign_withdrawal(
    //     public_key,
    //     network_magic,
    //     args.user_id,
    //     destination,
    //     args.value,
    //     args.nonce,
    // )?;

    // provider.add_withdrawal::<F>(req).await?;

    // Ok(())

    unimplemented!()
}
