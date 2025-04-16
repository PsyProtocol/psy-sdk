use std::time::Duration;

use anyhow::Result;
use qed_common::link::data::BTCAddress160;
use qed_core::data::base_types::hash256::Hash256;
use qed_crypto::signature::secp256k1::wallet::MemorySecp256K1Wallet;

use crate::rpc::provider::RpcProvider;

use super::args::L1DepositArgs;
const MAX_CHECKPOINT_ID: u64 = 0xffffffff;

pub async fn run(args: L1DepositArgs) -> Result<()> {
    // let provider = RpcProvider::new(&args.rpc_address);
    // let mut wallet = MemorySecp256K1Wallet::new();
    // let api = BTCLinkAPI::new_str(&args.bitcoin_rpc, &args.electrs_api);
    // let from = BTCAddress160::from_p2pkh_key(
    //     wallet.add_private_key(Hash256::from_hex_string(&args.private_key)?)?,
    // );

    // let deposit_address = if args.deposit_address.is_empty() {
    //     provider
    //         .get_city_block_deposit_address(MAX_CHECKPOINT_ID)
    //         .await?
    // } else {
    //     BTCAddress160::try_from_string(&args.deposit_address)?.address
    // };
    // let txid = api.fund_address_from_known_p2pkh_address(
    //     &wallet,
    //     from,
    //     BTCAddress160::new_p2sh(deposit_address),
    //     args.amount,
    // )?;
    // if api.is_regtest() {
    //     // make sure to mine some blocks so the address is indexed by electrs
    //     tokio::time::sleep(Duration::from_millis(500)).await;
    //     api.mine_blocks(10)?;
    //     tokio::time::sleep(Duration::from_millis(300)).await;
    //     api.mine_blocks(10)?;
    //     tokio::time::sleep(Duration::from_millis(200)).await;
    //     api.mine_blocks(10)?;
    // }
    // println!("{{\"txid\": \"{}\"}}", txid.to_hex_string());
    // Ok(())

    unimplemented!()
}
