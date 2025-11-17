use anyhow::Result;
use psy_common::args::SignType;

use super::{args::GetPublicKeyArgs, key_utils::load_wallet_key_info};

pub async fn run(args: GetPublicKeyArgs) -> Result<()> {
    let info = load_wallet_key_info(&args.wallet, false)?;
    println!("sign_type: {:?}", info.sign_type);
    println!("fingerprint: {}", info.fingerprint);
    println!("public_key_param: {}", info.public_key_param);
    println!("public_key: {}", info.public_key_hash);
    println!("private_key: {}", info.private_key);
    Ok(())
}
