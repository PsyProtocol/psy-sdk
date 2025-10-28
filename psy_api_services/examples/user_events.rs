//! User Events Telemetry Example: /telemetry/events → /user_events → /user_events_aggregations
//!
//! This example demonstrates the complete workflow:
//! 1. Send user events via /telemetry/events
//! 2. Query user events using /user_events
//! 3. Query aggregated data using /user_events_aggregations

use chrono::Utc;
use reqwest::Client;
use serde_json::json;

const API_BASE: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let now = Utc::now();

    // Send user events via telemetry API
    println!("🚀 Sending user events via telemetry...");

    // Create sample user events with different transaction types
    let telemetry_payload = json!({
        "user_events": [
            {
                "user_id": "user_001",
                "public_key": "0x1234567890abcdef1234567890abcdef12345678",
                "tx_type": "REGISTER_USER",
                "metadata": {
                    "twitter_handle": "alice_qed",
                    "label": "Alpha Tester",
                    "registration_method": "web"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "user_id": "user_002",
                "public_key": "0xfedcba0987654321fedcba0987654321fedcba09",
                "tx_type": "DEPLOY_CONTRACT",
                "metadata": {
                    "contract_name": "SimpleToken",
                    "contract_type": "erc20",
                    "bytecode_size": 2048,
                    "gas_limit": 500000
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "user_id": "user_003",
                "public_key": "0xabcdef1234567890abcdef1234567890abcdef12",
                "tx_type": "GUTA",
                "metadata": {
                    "guta_type": "transfer",
                    "amount": "1000000000000000000",
                    "recipient": "0x9876543210fedcba9876543210fedcba98765432",
                    "gas_used": 21000
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "user_id": "user_001",
                "public_key": "0x1234567890abcdef1234567890abcdef12345678",
                "tx_type": "DEPLOY_CONTRACT",
                "metadata": {
                    "contract_name": "MultiSig",
                    "contract_type": "wallet",
                    "bytecode_size": 4096,
                    "gas_limit": 800000,
                    "signers": ["0x1234567890abcdef1234567890abcdef12345678", "0xfedcba0987654321fedcba0987654321fedcba09"]
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "user_id": "user_004",
                "public_key": "0x567890abcdef1234567890abcdef1234567890ab",
                "tx_type": "REGISTER_USER",
                "metadata": {
                    "twitter_handle": "bob_blockchain",
                    "label": "Developer",
                    "registration_method": "cli"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            }
        ]
    });

    let response = client
        .post(&format!("{}/telemetry/events", API_BASE))
        .json(&telemetry_payload)
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    println!(
        "✅ Telemetry response: {}",
        serde_json::to_string_pretty(&body)?
    );

    // Add a small delay to ensure data is processed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Query user events
    println!("\n📊 Querying user events...");

    // Query all user events
    println!("🔸 All user events");
    let response = client
        .get(&format!("{}/user_events", API_BASE))
        .send()
        .await?;
    let all_events: serde_json::Value = response.json().await?;
    println!(
        "All user events count: {}",
        all_events.as_array().unwrap().len()
    );

    // Query by user_id
    println!("\n🔸 User events for user_id=user_001");
    let response = client
        .get(&format!("{}/user_events?user_id=user_001", API_BASE))
        .send()
        .await?;
    let user_events: serde_json::Value = response.json().await?;
    println!(
        "User 001 events count: {}",
        user_events.as_array().unwrap().len()
    );

    // Query by transaction type
    println!("\n🔸 REGISTER_USER events");
    let response = client
        .get(&format!("{}/user_events?tx_type=REGISTER_USER", API_BASE))
        .send()
        .await?;
    let register_events: serde_json::Value = response.json().await?;
    println!(
        "Register user events count: {}",
        register_events.as_array().unwrap().len()
    );

    // Query by time range
    let start_time = now - chrono::Duration::minutes(5);
    let end_time = now + chrono::Duration::minutes(5);
    println!(
        "\n🔸 User events in time range from {} to {}",
        start_time, end_time
    );
    let response = client
        .get(&format!(
            "{}/user_events?start_time={}&end_time={}",
            API_BASE, start_time, end_time
        ))
        .send()
        .await?;
    let time_filtered_events: serde_json::Value = response.json().await?;
    println!(
        "Time filtered events count: {}",
        time_filtered_events.as_array().unwrap().len()
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Query aggregated data 📈
    println!("\n📈 Querying user events aggregations...");

    // Note: Aggregations may not show data immediately due to TimescaleDB continuous aggregation refresh policies
    // In a real scenario, data would be aggregated over longer time periods (hours/days)

    let aggregation_buckets = vec!["1h", "1d", "1w"];

    for bucket in aggregation_buckets {
        println!(
            "\n🔸 {}: User events aggregation ({})",
            match bucket {
                "1h" => 1,
                "1d" => 2,
                _ => 3,
            },
            bucket
        );

        let response = client
            .get(&format!(
                "{}/user_events_aggregations?end_time={}&bucket={}",
                API_BASE, end_time, bucket
            ))
            .send()
            .await?;

        let aggregations: serde_json::Value = response.json().await?;
        println!(
            "📊 {} aggregations: {}",
            bucket,
            serde_json::to_string_pretty(&aggregations)?
        );
    }

    // Display summary statistics 📋
    println!("\n📋 Summary Statistics");
    let response = client.get(&format!("{}/stats", API_BASE)).send().await?;
    let stats: serde_json::Value = response.json().await?;
    println!("📊 Global stats: {}", serde_json::to_string_pretty(&stats)?);

    println!("\n🎉 User events telemetry example completed successfully!");
    println!(
        "💡 To see aggregation data, run this example multiple times or wait for longer periods."
    );

    Ok(())
}
