use std::{process::Stdio, time::Duration};
use tokio::{process::Command, time::sleep};

use hex::encode;
use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::PrimeField64;
use qed_core::data::qhashout::QHashOut;

#[tokio::main]
async fn main() -> Result<()> {
    //number of users
    let count = 100;

    for i in 0..count {
        let priv_key = QHashOut::<GoldilocksField>::rand();
        let bytes: Vec<u8> = priv_key
            .0
            .elements
            .iter()
            .flat_map(|x| x.to_canonical_u64().to_le_bytes())
            .collect();
        let priv_key_hex = encode(bytes);

        println!("🔑 [{}] registering user with priv_key = {}", i, priv_key_hex);

        let mut child = Command::new("qed_user_cli")
            .arg("register-user")
            .arg("--private-key")
            .arg(&priv_key_hex)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let output = child.wait_with_output().await?;
        println!("📨 [{}] output: {}", i, String::from_utf8_lossy(&output.stdout));

        if !output.status.success() {
            eprintln!("❌ [{}] error: {}", i, String::from_utf8_lossy(&output.stderr));
        }

        sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}