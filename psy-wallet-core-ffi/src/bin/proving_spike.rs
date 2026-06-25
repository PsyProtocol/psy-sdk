//! Proving spike — the #1 de-risking step for native mobile.
//!
//! Measures how long `register_user` (a REAL Plonky2 ZK-signature proof) takes
//! through the exact same `PsyWalletCore` API the iOS/Android apps call. Run it
//! on desktop first for a baseline; the on-device harness (a thin iOS/Android
//! test target) calls the SAME uniffi method and reports time + peak memory,
//! which tells us whether Plonky2 proving fits a phone's memory budget BEFORE
//! any UI is built.
//!
//! Run:
//!   PSY_CONFIG=/path/to/config.json \
//!   PSY_KEY=<64-hex bare lowercase> PSY_SIGN_TYPE=zk \
//!   cargo run --release --bin proving_spike
use std::time::Instant;

use psy_wallet_core_ffi::PsyWalletCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::var("PSY_CONFIG").unwrap_or_else(|_| "config.json".into());
    let config_json = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("read PSY_CONFIG ({config_path}): {e}"))?;

    // Throwaway test key — bare 64-hex, lowercase. NEVER a real-funds key.
    let private_key = std::env::var("PSY_KEY").unwrap_or_else(|_| {
        "1111111111111111111111111111111111111111111111111111111111111111".into()
    });
    let sign_type = std::env::var("PSY_SIGN_TYPE").unwrap_or_else(|_| "zk".into());

    eprintln!("[spike] building wallet core (RPC bootstrap)…");
    let t0 = Instant::now();
    let core = PsyWalletCore::new(config_json).await?;
    eprintln!("[spike] core ready in {:?}", t0.elapsed());

    eprintln!("[spike] register_user — proving the ZK-signature circuit…");
    let t1 = Instant::now();
    let pk_hash = core.register_user(private_key, sign_type).await?;
    let dur = t1.elapsed();

    println!("[spike] RESULT register_user proof = {dur:?}  pkHash={pk_hash}");
    println!("[spike] (run again with `/usr/bin/time -l` (macOS) or `-v` (Linux) for peak RSS)");
    Ok(())
}
