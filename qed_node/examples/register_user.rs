use std::str::FromStr;
use anyhow::Result;
use rand::Rng;
use reqwest::Client;
use std::time::{Duration, Instant};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_prover::{local::request::{Id, QRegisterUserRPCRequest, RequestParams, RpcRequest, Version}, wallet::memory_wallet::ZK_FINGERPRINT};

//just copy from qed_user_cli/src/subcommand/register_user.rs
// const ZK_FINGERPRINT: &str = "d2f572f1402fa8a92c9af0a2226e05ef8f5f4f34d764c6515b90d2b391fc48c1";

// Generate a random public key using thread_rng for better performance
#[inline(always)]
fn generate_random_public_key() -> ZKPublicKeyInfo<GoldilocksField> {
    let mut rng = rand::thread_rng();
    let fingerprint = QHashOut::<GoldilocksField>::from_str(&ZK_FINGERPRINT).unwrap();

    let u64v = [rng.r#gen::<u64>(), rng.r#gen::<u64>(), rng.r#gen::<u64>(), rng.r#gen::<u64>()];
    let public_key_param = QHashOut::<GoldilocksField>::from_values(
        u64v[0],
        u64v[1],
        u64v[2],
        u64v[3],
    );

    let public_key_info = ZKPublicKeyInfo {
        fingerprint,
        public_key_param,
    };
    public_key_info
}

async fn register_user(idx: u64, url: &str, client: &Client) -> Result<()> {
    let test_public_key = generate_random_public_key();

    let test_request = RpcRequest {
        jsonrpc: Version::V2,
        request: RequestParams::RegisterUser(QRegisterUserRPCRequest {
            public_key: test_public_key,
        }),
        id: Id::Number(0),
    };

    println!("\n🚀 {} Sending test request...", idx);
    let start = Instant::now();

    let response = client
        .post(url)
        .json(&test_request)
        .send()
        .await?;

    let duration = start.elapsed();

    if response.status().is_success() {
        println!("\n✅ {} register user success,  response time: {:.3}ms", idx, duration.as_millis());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "Unable to read body".to_string());
        println!("\n❌ {} register user FAILED!", idx);
        println!("        HTTP Status: {}", status);
        println!("        Response: {}", body);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        Err(anyhow::anyhow!("Connection test failed"))
    }
}

async fn run_register_loop() -> Result<()> {
    let rpc_url = "http://127.0.0.1:8545".to_string();
    let mut counter = 0;

    let test_client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    loop {
        tokio::time::sleep(Duration::from_millis(3000)).await;

        // Handle the result without exiting on failure
        match register_user(counter, &rpc_url, &test_client).await {
            Ok(_) => {
                // Success - continue normally
                counter += 1;
            }
            Err(e) => {
                // Failure - log the error, wait 5 seconds, then continue
                println!("⚠️  Error occurred: {}", e);
                println!("⏳ Waiting 5 seconds before retrying...\n");
                tokio::time::sleep(Duration::from_secs(5)).await;
                // Note: counter is NOT incremented on failure, so it will retry with the same index
                // If you want to increment counter even on failure, move counter += 1 here
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();

    tokio::select! {
        result = run_register_loop() => {
            match result {
                Ok(()) => println!("🔰 Register user loop completed."),
                Err(e) => println!("❌ Register user loop error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            println!("🔰 Ctrl-C received, shutting down gracefully...");
        }
    }

    Ok(())
}
