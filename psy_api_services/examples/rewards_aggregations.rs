//! Worker Rewards Aggregations API Example: /rewards_aggregations/{worker_public_key}
//!
//! This example demonstrates the worker rewards aggregations system:
//! 1. Create sample worker events with COMPLETED GUTA circuit types across different time periods
//! 2. Background reward processing automatically calculates rewards and continuous aggregates
//! 3. Query worker rewards aggregations using /rewards_aggregations/{worker_public_key}
//! 4. Test different time buckets (1d, 1w, 1m) and time ranges
//!
//! Note: This API uses TimescaleDB continuous aggregates to provide pre-computed time-series data:
//! - worker_rewards_1d: Daily aggregations
//! - worker_rewards_1w: Weekly aggregations
//! - worker_rewards_1m: Monthly aggregations

use chrono::{Duration, Utc};
use psy_api_services::models::WorkerRewardsAggregation;
use psy_core::job::id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID};
use reqwest::Client;
use serde_json::json;

const API_BASE: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let now = Utc::now();

    println!("📊 QED Worker Rewards Aggregations API Example");
    println!("🚀 Creating sample worker events across different time periods...");
    println!("⏳ Background reward processing will create continuous aggregates");

    // Define test worker
    let worker_key = "rewards_agg_test_0xabcdef123456789";

    // Create worker events across different time periods to demonstrate aggregations
    let telemetry_payload = json!({
        "worker_events": [
            // Today - Multiple completed GUTA proofs
            {
                "realm_id": 0,
                "public_key": worker_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_guta_job_id("job_today_001", "GUTATwoEndCap"),
                "checkpoint_id": 100,
                "duration": 12000,
                "metadata": {
                    "task_type": "guta_processing",
                    "circuit_type": "GUTATwoEndCap"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "realm_id": 0,
                "public_key": worker_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_guta_job_id("job_today_002", "GUTARegisterUsers"),
                "checkpoint_id": 105,
                "duration": 15000,
                "metadata": {
                    "task_type": "guta_processing",
                    "circuit_type": "GUTARegisterUsers"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            // Yesterday - Some completed proofs
            {
                "realm_id": 0,
                "public_key": worker_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_guta_job_id("job_yesterday_001", "GUTASingleEndCap"),
                "checkpoint_id": 95,
                "duration": 18000,
                "metadata": {
                    "task_type": "guta_processing",
                    "circuit_type": "GUTASingleEndCap"
                },
                "timestamp": (now - Duration::days(1)).to_rfc3339(),
                "created_at": (now - Duration::days(1)).to_rfc3339(),
                "updated_at": (now - Duration::days(1)).to_rfc3339()
            },
            // 5 days ago - More completed proofs
            {
                "realm_id": 0,
                "public_key": worker_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_guta_job_id("job_5days_001", "GUTAVerifyToCap"),
                "checkpoint_id": 80,
                "duration": 14000,
                "metadata": {
                    "task_type": "guta_processing",
                    "circuit_type": "GUTAVerifyToCap"
                },
                "timestamp": (now - Duration::days(5)).to_rfc3339(),
                "created_at": (now - Duration::days(5)).to_rfc3339(),
                "updated_at": (now - Duration::days(5)).to_rfc3339()
            },
            {
                "realm_id": 0,
                "public_key": worker_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_guta_job_id("job_5days_002", "GUTAOnlyRegisterUsers"),
                "checkpoint_id": 85,
                "duration": 16000,
                "metadata": {
                    "task_type": "guta_processing",
                    "circuit_type": "GUTAOnlyRegisterUsers"
                },
                "timestamp": (now - Duration::days(5)).to_rfc3339(),
                "created_at": (now - Duration::days(5)).to_rfc3339(),
                "updated_at": (now - Duration::days(5)).to_rfc3339()
            },
            // 15 days ago - Historical data for weekly aggregations
            {
                "realm_id": 0,
                "public_key": worker_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_guta_job_id("job_15days_001", "GUTANoChange"),
                "checkpoint_id": 60,
                "duration": 11000,
                "metadata": {
                    "task_type": "guta_processing",
                    "circuit_type": "GUTANoChange"
                },
                "timestamp": (now - Duration::days(15)).to_rfc3339(),
                "created_at": (now - Duration::days(15)).to_rfc3339(),
                "updated_at": (now - Duration::days(15)).to_rfc3339()
            },
            // 40 days ago - Historical data for monthly aggregations
            {
                "realm_id": 0,
                "public_key": worker_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_guta_job_id("job_40days_001", "GUTATwoGUTA"),
                "checkpoint_id": 30,
                "duration": 20000,
                "metadata": {
                    "task_type": "guta_processing",
                    "circuit_type": "GUTATwoGUTA"
                },
                "timestamp": (now - Duration::days(40)).to_rfc3339(),
                "created_at": (now - Duration::days(40)).to_rfc3339(),
                "updated_at": (now - Duration::days(40)).to_rfc3339()
            }
        ]
    });

    let response = client
        .post(&format!("{}/telemetry/events", API_BASE))
        .json(&telemetry_payload)
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    println!("✅ Sample worker events created successfully");
    println!(
        "📊 Events created: {}",
        body.get("events_processed").unwrap_or(&json!(0))
    );

    // Wait for background reward processing and continuous aggregate updates
    println!("⏳ Waiting 20 seconds for reward processing and continuous aggregates to update...");
    tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;

    // Test different aggregation buckets and time ranges
    println!("\n📊 Testing different aggregation buckets...");

    let test_scenarios = vec![
        (
            "1d",
            None,
            None,
            Some(10),
            "Daily aggregations (last 10 days)",
        ),
        (
            "1w",
            None,
            None,
            Some(6),
            "Weekly aggregations (last 6 weeks)",
        ),
        (
            "1m",
            None,
            None,
            Some(3),
            "Monthly aggregations (last 3 months)",
        ),
        (
            "1d",
            Some(now - Duration::days(7)),
            Some(now),
            Some(10),
            "Daily aggregations for last 7 days",
        ),
    ];

    for (bucket, start_time, end_time, limit, description) in test_scenarios {
        println!("\n🔸 {}", description);

        let mut url = format!(
            "{}/rewards_aggregations/{}?bucket={}",
            API_BASE, worker_key, bucket
        );

        if let Some(start) = start_time {
            url.push_str(&format!("&start_time={}", start.to_rfc3339()));
        }

        if let Some(end) = end_time {
            url.push_str(&format!("&end_time={}", end.to_rfc3339()));
        }

        if let Some(lmt) = limit {
            url.push_str(&format!("&limit={}", lmt));
        }

        println!("🌐 Request URL: {}", url);

        let response = client.get(&url).send().await?;

        match response.status().as_u16() {
            200 => {
                let aggregations: Vec<WorkerRewardsAggregation> = response.json().await?;
                println!(
                    "✅ Success! Retrieved {} aggregation periods",
                    aggregations.len()
                );

                if aggregations.is_empty() {
                    println!("   📭 No aggregation data found for this time period");
                } else {
                    println!("📈 Aggregation Summary:");
                    let mut total_proofs = 0;
                    let mut total_rewards = 0;

                    for (i, agg) in aggregations.iter().enumerate() {
                        if i < 5 {
                            // Show first 5 entries
                            println!(
                                "   {}. Bucket: {}",
                                i + 1,
                                agg.bucket.format("%Y-%m-%d %H:%M:%S")
                            );
                            println!("      Worker: {}", agg.public_key);
                            println!(
                                "      Proofs: {} | Rewards: {} psy | Max Checkpoint: {}",
                                agg.completed_proofs, agg.total_rewards, agg.max_checkpoint
                            );
                        }
                        total_proofs += agg.completed_proofs;
                        total_rewards += agg.total_rewards;
                    }

                    if aggregations.len() > 5 {
                        println!("   ... and {} more periods", aggregations.len() - 5);
                    }

                    println!("📊 Period Totals:");
                    println!("   • Total Proofs: {}", total_proofs);
                    println!("   • Total Rewards: {} psy", total_rewards);
                    if total_proofs > 0 {
                        println!(
                            "   • Avg Rewards per Proof: {} psy",
                            total_rewards / total_proofs
                        );
                    }

                    // Validate data consistency
                    for agg in &aggregations {
                        assert_eq!(
                            agg.total_rewards,
                            agg.completed_proofs * 5_000_000_000,
                            "Rewards calculation should be 5×10⁹ psy per completed proof"
                        );
                    }
                    println!("   ✅ All reward calculations verified!");
                }
            }
            400 => {
                println!("❌ Bad Request - Invalid bucket parameter or other query issue");
            }
            500 => {
                println!("❌ Internal Server Error - Check server logs");
            }
            status => {
                println!("❌ Unexpected status code: {}", status);
            }
        }
    }

    // Test invalid bucket parameter
    println!("\n🧪 Testing invalid bucket parameter...");
    let response = client
        .get(&format!(
            "{}/rewards_aggregations/{}?bucket=invalid",
            API_BASE, worker_key
        ))
        .send()
        .await?;

    println!("Response status for invalid bucket: {}", response.status());
    assert_eq!(
        response.status().as_u16(),
        400,
        "Should return 400 Bad Request for invalid bucket"
    );
    println!("✅ Invalid bucket parameter handled correctly");

    println!("\n🎯 Key Features Demonstrated:");
    println!("   ✓ Time-series aggregations: Daily (1d), Weekly (1w), Monthly (1m)");
    println!("   ✓ TimescaleDB continuous aggregates for efficient queries");
    println!("   ✓ Flexible time range filtering with start_time/end_time");
    println!("   ✓ Configurable result limits (default 100, max 1000)");
    println!("   ✓ Aggregated metrics: completed_proofs, total_rewards, max_checkpoint");
    println!("   ✓ Time bucket information for time-series analysis");
    println!("   ✓ Error handling for invalid parameters");

    println!("\n📋 API Parameters:");
    println!("   • bucket (required): '1d', '1w', or '1m'");
    println!("   • start_time (optional): RFC3339 timestamp");
    println!("   • end_time (optional): RFC3339 timestamp");
    println!("   • limit (optional): Number of periods to return (1-1000, default 100)");

    println!("\n🎉 Worker rewards aggregations API example completed successfully!");
    println!("💡 Tip: Use different bucket sizes for different analysis needs:");
    println!("   • 1d: Detailed daily performance tracking");
    println!("   • 1w: Weekly trend analysis");
    println!("   • 1m: Long-term monthly performance");

    Ok(())
}

/// Helper function to create a sample QProvingJobDataID with GUTA circuit types
fn create_guta_job_id(_job_name: &str, circuit_type_name: &str) -> serde_json::Value {
    // Map circuit type name to the appropriate enum (actual GUTA types from the enum)
    let circuit_type = match circuit_type_name {
        "GUTATwoEndCap" => ProvingJobCircuitType::GUTATwoEndCap,
        "GUTATwoGUTA" => ProvingJobCircuitType::GUTATwoGUTA,
        "GUTALeftEndCapRightGUTA" => ProvingJobCircuitType::GUTALeftEndCapRightGUTA,
        "GUTALeftGUTARightEndCap" => ProvingJobCircuitType::GUTALeftGUTARightEndCap,
        "GUTASingleEndCap" => ProvingJobCircuitType::GUTASingleEndCap,
        "GUTARegisterUsers" => ProvingJobCircuitType::GUTARegisterUsers,
        "GUTAVerifyToCap" => ProvingJobCircuitType::GUTAVerifyToCap,
        "GUTAOnlyRegisterUsers" => ProvingJobCircuitType::GUTAOnlyRegisterUsers,
        "GUTANoChange" => ProvingJobCircuitType::GUTANoChange,
        "GUTATwoGUTAWithCheckpointUpgrade" => ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade,
        "GUTAVerifyToCapWithCheckpointUpgrade" => ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade,
        _ => ProvingJobCircuitType::GUTATwoEndCap, // Default fallback
    };

    let job = QProvingJobDataID {
        topic: QJobTopic::GenerateStandardProof,
        goal_id: 0,
        slot_id: 0,
        circuit_type, // Only GUTA circuit types count for rewards
        group_id: 0,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType::OutputProof,
        data_index: 0,
    };
    serde_json::to_value(job).unwrap()
}
