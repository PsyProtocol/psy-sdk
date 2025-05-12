
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use reqwest::Client;
use plonky2::field::goldilocks_field::GoldilocksField as QEDFelt;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::HashOut;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;

const DO_QUERY: bool = true;
const USERS_PER_BATCH: u64 = 10;
const SLEEP_AFTER_BATCH_SECS: u64 = 10;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new();
    let ce_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8545".to_string());

    let mut global_counter = 1u64;

    loop {
        let mut current_batch = Vec::new();

        for i in 0..USERS_PER_BATCH {
            let fingerprint = QHashOut(HashOut {
                elements: [
                    QEDFelt::from_canonical_u64(global_counter + 1),
                    QEDFelt::from_canonical_u64(global_counter + 1),
                    QEDFelt::from_canonical_u64(global_counter + 1),
                    QEDFelt::from_canonical_u64(global_counter + 1),
                ],
            });

            let public_key_param = QHashOut(HashOut {
                elements: [
                    QEDFelt::from_canonical_u64(global_counter),
                    QEDFelt::from_canonical_u64(global_counter),
                    QEDFelt::from_canonical_u64(global_counter),
                    QEDFelt::from_canonical_u64(global_counter),
                ],
            });

            let zk_public_key = ZKPublicKeyInfo {
                fingerprint,
                public_key_param,
            };

            let payload = json!({
                "jsonrpc": "2.0",
                "id": global_counter,
                "method": "qed_register_user",
                "params": zk_public_key,
            });

            let res = client.post(&ce_url).json(&payload).send().await;

            match res {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_else(|_| "<no body>".to_string());
                    println!("✅ [{}] Register response: {}", global_counter, text);
                    current_batch.push(public_key_param);
                }
                Err(err) => {
                    eprintln!("❌ [{}] Registration failed: {:?}", global_counter, err);
                }
            }

            global_counter += 1;
            sleep(Duration::from_millis(100)).await;
        }

        println!("⏳ Sleeping {}s before query...", SLEEP_AFTER_BATCH_SECS);
        sleep(Duration::from_secs(SLEEP_AFTER_BATCH_SECS)).await;

        // let fingerprint = QHashOut(HashOut {
        //     elements: [
        //         QEDFelt::from_canonical_u64(counter + 1),
        //         QEDFelt::from_canonical_u64(counter + 1 ),
        //         QEDFelt::from_canonical_u64(counter + 1),
        //         QEDFelt::from_canonical_u64(counter + 1),
        //     ],
        // });
        //
        // let public_key_param = QHashOut(HashOut {
        //     elements: [
        //         QEDFelt::from_canonical_u64(counter),
        //         QEDFelt::from_canonical_u64(counter),
        //         QEDFelt::from_canonical_u64(counter),
        //         QEDFelt::from_canonical_u64(counter),
        //     ],
        // });
        //
        // let zk_public_key = ZKPublicKeyInfo {
        //     fingerprint,
        //     public_key_param,
        // };
        //
        // let payload = json!({
        //     "jsonrpc": "2.0",
        //     "id": 1,
        //     "method": "qed_register_user",
        //       "params": zk_public_key,
        // });
        //
        // let res = client
        //     .post(&ce_url)
        //     .json(&payload)
        //     .send()
        //     .await;
        //
        // match res {
        //     Ok(resp) => {
        //         let text = resp.text().await.unwrap_or_else(|_| "<no body>".to_string());
        //         println!("✅ {} Response: {}", counter, text);
        //     }
        //     Err(err) => {
        //         eprintln!("❌ {} Error sending request: {:?}", counter, err);
        //     }
        // }

        // Wait 5 seconds
        // sleep(Duration::from_secs(10)).await;
        if DO_QUERY {
            println!("🔍 Querying user_ids for current batch...");
            for (i, public_key_param) in current_batch.iter().enumerate() {
                let payload = json!({
                    "jsonrpc": "2.0",
                    "id": 100_000 + global_counter + i as u64,
                    "method": "qed_get_user_id",
                    "params": public_key_param,
                });

                let res = client.post(&ce_url).json(&payload).send().await;
                match res {
                    Ok(resp) => {
                        let text = resp.text().await.unwrap_or_else(|_| "<no body>".to_string());
                        println!("🔍 [{}] Query result: {}", global_counter + i as u64, text);
                    }
                    Err(err) => {
                        eprintln!("❌ [{}] Query failed: {:?}", global_counter + i as u64, err);
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }
        }
        println!("🔁 Batch complete. Looping...\n");

    }
}
