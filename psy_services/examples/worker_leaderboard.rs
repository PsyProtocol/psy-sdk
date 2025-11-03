// Example program to test Worker Leaderboard API
use reqwest;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Worker Leaderboard API...");

    let client = reqwest::Client::new();

    // Test with default limit (100)
    println!("\n📊 Testing default limit (100 workers):");
    let url = "http://localhost:3000/leaderboard/workers";

    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status();
            println!("Status: {}", status);

            if status.is_success() {
                let body = response.text().await?;
                let json: Value = serde_json::from_str(&body)?;

                if let Some(array) = json.as_array() {
                    println!("✅ Successfully retrieved {} worker entries", array.len());

                    // Show top 3 entries
                    for (i, entry) in array.iter().take(3).enumerate() {
                        if let Some(obj) = entry.as_object() {
                            let rank = obj.get("rank").and_then(|v| v.as_i64()).unwrap_or(0);
                            let worker_key = obj.get("worker_public_key").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let twitter = obj.get("twitter_username").and_then(|v| v.as_str()).unwrap_or("No Twitter");
                            let proofs = obj.get("proofs_24h").and_then(|v| v.as_i64()).unwrap_or(0);
                            let rewards = obj.get("rewards_24h").and_then(|v| v.as_i64()).unwrap_or(0);

                            println!(
                                "  #{}: {} | Twitter: @{} | Proofs: {} | Rewards: {} PSY",
                                rank,
                                &worker_key[..8.min(worker_key.len())],
                                twitter,
                                proofs,
                                rewards
                            );
                        }
                    }
                } else {
                    println!("❌ Response is not an array");
                }
            } else {
                let error_body = response.text().await?;
                println!("❌ Request failed: {}", error_body);
            }
        }
        Err(e) => {
            println!("❌ Network error: {}", e);
        }
    }

    // Test with custom limit (10)
    println!("\n📊 Testing custom limit (10 workers):");
    let url_with_limit = "http://localhost:3000/leaderboard/workers?limit=10";

    match client.get(url_with_limit).send().await {
        Ok(response) => {
            let status = response.status();
            println!("Status: {}", status);

            if status.is_success() {
                let body = response.text().await?;
                let json: Value = serde_json::from_str(&body)?;

                if let Some(array) = json.as_array() {
                    println!("✅ Successfully retrieved {} worker entries with limit=10", array.len());
                } else {
                    println!("❌ Response is not an array");
                }
            } else {
                let error_body = response.text().await?;
                println!("❌ Request failed: {}", error_body);
            }
        }
        Err(e) => {
            println!("❌ Network error: {}", e);
        }
    }

    println!("\n🎯 Worker Leaderboard API test completed!");

    Ok(())
}
