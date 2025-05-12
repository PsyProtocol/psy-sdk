
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new();
    let ce_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8545".to_string());

    let mut counter = 1u64;

    loop {
        let fingerprint = QHashOut(HashOut {
            elements: [
                QEDFelt::from_canonical_u64(counter + 1),
                QEDFelt::from_canonical_u64(counter + 1 ),
                QEDFelt::from_canonical_u64(counter + 1),
                QEDFelt::from_canonical_u64(counter + 1),
            ],
        });

        let public_key_param = QHashOut(HashOut {
            elements: [
                QEDFelt::from_canonical_u64(counter),
                QEDFelt::from_canonical_u64(counter),
                QEDFelt::from_canonical_u64(counter),
                QEDFelt::from_canonical_u64(counter),
            ],
        });

        let zk_public_key = ZKPublicKeyInfo {
            fingerprint,
            public_key_param,
        };

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "qed_register_user",
              "params": zk_public_key,
        });

        let res = client
            .post(&ce_url)
            .json(&payload)
            .send()
            .await;

        match res {
            Ok(resp) => {
                let text = resp.text().await.unwrap_or_else(|_| "<no body>".to_string());
                println!("✅ {} Response: {}", counter, text);
            }
            Err(err) => {
                eprintln!("❌ {} Error sending request: {:?}", counter, err);
            }
        }

        counter += 1;
        sleep(Duration::from_millis(100)).await;
    }
}
