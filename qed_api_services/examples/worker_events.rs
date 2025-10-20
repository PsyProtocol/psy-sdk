//! Worker Events Telemetry Example: /telemetry/events → /worker_events → /worker_events_aggregations
//!
//! This example demonstrates the complete workflow:
//! 1. Send worker events via /telemetry/events
//! 2. Query worker events using /worker_events
//! 3. Query aggregated data using /worker_events_aggregations

use chrono::Utc;
use qed_core::job::id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID};
use reqwest::Client;
use serde_json::json;

const API_BASE: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let now = Utc::now();

    // Send worker events via telemetry API
    println!("🚀 Sending worker events via telemetry...");

    // Create sample worker events with different statuses and realms
    let telemetry_payload = json!({
        "worker_events": [
            {
                "realm_id": 0,
                "public_key": "worker1_key_0xabcdef123456789",
                "status": "PENDING",
                "source": "REALM",
                "job_id": create_sample_job_id("job_001"),
                "checkpoint_id": 100,
                "duration": null,
                "metadata": {
                    "task_type": "zk_proof_generation",
                    "circuit_type": "coordinator"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "realm_id": 0,
                "public_key": "worker2_key_0x987654321abcdef",
                "status": "PROCESSING",
                "source": "REALM",
                "job_id": create_sample_job_id("job_002"),
                "checkpoint_id": 101,
                "duration": null,
                "metadata": {
                    "task_type": "zk_proof_generation",
                    "circuit_type": "guta"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "realm_id": 16384,
                "public_key": "worker3_key_0xfedcba987654321",
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_sample_job_id("job_003"),
                "checkpoint_id": 102,
                "duration": 15000, // 15 seconds
                "metadata": {
                    "task_type": "zk_proof_generation",
                    "circuit_type": "ups",
                    "proof_size": 2048
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "realm_id": 16384,
                "public_key": "worker4_key_0x123abc789def456",
                "status": "FAILED",
                "source": "COORDINATOR",
                "job_id": create_sample_job_id("job_004"),
                "checkpoint_id": 103,
                "duration": 8000, // 8 seconds before failure
                "metadata": {
                    "task_type": "zk_proof_generation",
                    "circuit_type": "coordinator",
                    "error": "circuit_constraint_violation"
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

    // Query worker events
    println!("\n📊 Querying worker events...");

    // Query all worker events
    println!("🔸 All worker events");
    let response = client
        .get(&format!("{}/worker_events", API_BASE))
        .send()
        .await?;
    let all_events: serde_json::Value = response.json().await?;
    println!(
        "All worker events count: {}",
        all_events.as_array().unwrap().len()
    );

    // Query by realm_id
    println!("\n🔸 Worker events for realm_id=0");
    let response = client
        .get(&format!("{}/worker_events?realm_id=0", API_BASE))
        .send()
        .await?;
    let realm_events: serde_json::Value = response.json().await?;
    println!(
        "Realm 0 events count: {}",
        realm_events.as_array().unwrap().len()
    );

    // Query by status
    println!("\n🔸 Completed worker events");
    let response = client
        .get(&format!("{}/worker_events?status=COMPLETED", API_BASE))
        .send()
        .await?;
    let completed_events: serde_json::Value = response.json().await?;
    println!(
        "Completed events count: {}",
        completed_events.as_array().unwrap().len()
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    // Query by time range
    let start_time = now - chrono::Duration::minutes(5);
    let end_time = now + chrono::Duration::minutes(5);
    println!(
        "\n🔸 Worker events in time range from {} to {}",
        start_time, end_time
    );
    let response = client
        .get(&format!(
            "{}/worker_events?start_time={}&end_time={}",
            API_BASE, start_time, end_time
        ))
        .send()
        .await?;
    let time_filtered_events: serde_json::Value = response.json().await?;
    println!(
        "Time filtered events count: {}",
        time_filtered_events.as_array().unwrap().len()
    );

    // Query aggregated data 📈
    println!("\n📈 Querying worker events aggregations...");

    // Note: Aggregations may not show data immediately due to TimescaleDB continuous aggregation refresh policies
    // In a real scenario, data would be aggregated over longer time periods (hours/days)

    let aggregation_buckets = vec!["1h", "1d", "1w"];

    for bucket in aggregation_buckets {
        println!(
            "\n🔸 {}: Worker events aggregation ({})",
            match bucket {
                "1h" => 1,
                "1d" => 2,
                _ => 3,
            },
            bucket
        );

        let response = client
            .get(&format!(
                "{}/worker_events_aggregations?end_time={}&bucket={}",
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

    // Step 4: Display summary statistics 📋
    println!("\n📋 Step 4: Summary Statistics");
    let response = client.get(&format!("{}/stats", API_BASE)).send().await?;
    let stats: serde_json::Value = response.json().await?;
    println!("📊 Global stats: {}", serde_json::to_string_pretty(&stats)?);

    println!("\n🎉 Worker events telemetry example completed successfully!");
    println!(
        "💡 To see aggregation data, run this example multiple times or wait for longer periods."
    );

    Ok(())
}

/// Helper function to create a sample QProvingJobDataID
fn create_sample_job_id(_job_name: &str) -> serde_json::Value {
    let job = QProvingJobDataID {
        topic: QJobTopic::GenerateStandardProof,
        goal_id: 0,
        slot_id: 0,
        circuit_type: ProvingJobCircuitType::UserEndCap,
        group_id: 0,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType::OutputProof,
        data_index: 0,
    };
    serde_json::to_value(job).unwrap()
}
