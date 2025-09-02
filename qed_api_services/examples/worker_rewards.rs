//! Worker Rewards API Example: /rewards/{worker_public_key}
//!
//! This example demonstrates the complete workflow:
//! 1. Create sample worker events with COMPLETED GenerateStandardProof jobs
//! 2. Query worker rewards using /rewards/{worker_public_key}
//! 3. Demonstrate claimed vs unclaimed rewards based on checkpoint_id

use chrono::Utc;
use qed_api_services::models::WorkerRewards;
use qed_core::job::id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID};
use reqwest::Client;
use serde_json::json;

const API_BASE: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let now = Utc::now();

    println!("💰 QED Worker Rewards API Example");
    println!("🚀 Creating sample worker events to demonstrate rewards calculation...");

    // Define test workers
    let worker1_key = "worker_rewards_test_0xabcdef123456789";
    let worker2_key = "worker_rewards_test_0x987654321abcdef";

    // Create sample worker events with COMPLETED GenerateStandardProof jobs
    let telemetry_payload = json!({
        "worker_events": [
            // Worker 1 - Multiple completed proofs with different checkpoint_ids
            {
                "realm_id": 0,
                "public_key": worker1_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_generate_standard_proof_job_id("job_001"),
                "checkpoint_id": 50, // This will be claimed when checkpoint > 50
                "duration": 12000,
                "metadata": {
                    "task_type": "generate_standard_proof",
                    "circuit_type": "coordinator"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "realm_id": 0,
                "public_key": worker1_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_generate_standard_proof_job_id("job_002"),
                "checkpoint_id": 75, // This will be claimed when checkpoint > 75
                "duration": 15000,
                "metadata": {
                    "task_type": "generate_standard_proof",
                    "circuit_type": "guta"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            {
                "realm_id": 0,
                "public_key": worker1_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_generate_standard_proof_job_id("job_003"),
                "checkpoint_id": 120, // This will be unclaimed when checkpoint <= 100
                "duration": 18000,
                "metadata": {
                    "task_type": "generate_standard_proof",
                    "circuit_type": "ups"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            // Worker 2 - Fewer completed proofs
            {
                "realm_id": 16384,
                "public_key": worker2_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_generate_standard_proof_job_id("job_004"),
                "checkpoint_id": 30,
                "duration": 10000,
                "metadata": {
                    "task_type": "generate_standard_proof",
                    "circuit_type": "coordinator"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            // Add some events that should NOT count for rewards
            // (different topic type)
            {
                "realm_id": 0,
                "public_key": worker1_key,
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_different_topic_job_id("job_005"),
                "checkpoint_id": 60,
                "duration": 8000,
                "metadata": {
                    "task_type": "other_task",
                    "circuit_type": "ups"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            },
            // (non-completed status)
            {
                "realm_id": 0,
                "public_key": worker1_key,
                "status": "FAILED",
                "source": "REALM",
                "job_id": create_generate_standard_proof_job_id("job_006"),
                "checkpoint_id": 65,
                "duration": 5000,
                "metadata": {
                    "task_type": "generate_standard_proof",
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
    println!("✅ Sample worker events created successfully");
    println!(
        "📊 Events created: {}",
        body.get("events_processed").unwrap_or(&json!(0))
    );

    // Add a small delay to ensure data is processed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Query worker rewards with different checkpoint_ids
    println!("\n💰 Querying worker rewards...");

    // Test different checkpoint scenarios
    let test_scenarios = vec![
        (
            worker1_key,
            100,
            "Worker 1 with checkpoint 100 (should have mixed claimed/unclaimed)",
        ),
        (
            worker1_key,
            50,
            "Worker 1 with checkpoint 50 (should have mostly unclaimed)",
        ),
        (
            worker1_key,
            150,
            "Worker 1 with checkpoint 150 (should have mostly claimed)",
        ),
        (
            worker2_key,
            50,
            "Worker 2 with checkpoint 50 (should have some claimed)",
        ),
    ];

    for (worker_key, checkpoint_id, description) in test_scenarios {
        println!("\n🔸 {}", description);
        let response = client
            .get(&format!(
                "{}/rewards/{}?checkpoint_id={}",
                API_BASE, worker_key, checkpoint_id
            ))
            .send()
            .await?;

        let rewards: WorkerRewards = response.json().await?;
        println!("💎 Worker: {}", worker_key);
        println!("📍 Checkpoint ID: {}", checkpoint_id);
        println!("🏆 Rewards: {}", serde_json::to_string_pretty(&rewards)?);

        // Calculate some insights using typed fields
        println!("💡 Analysis:");
        println!(
            "   • Claimed: {} proofs = {} psy",
            rewards.claimed_proofs, rewards.claimed_rewards
        );
        println!(
            "   • Unclaimed: {} proofs = {} psy",
            rewards.unclaimed_proofs, rewards.unclaimed_rewards
        );
        println!(
            "   • Total: {} proofs = {} psy",
            rewards.total_proofs, rewards.total_rewards
        );
        println!("   • Each proof = 5,000,000,000 psy (5×10⁹)");
        
        println!("⏰ Time-based Rewards:");
        println!(
            "   • Last 24 hours: {} psy",
            rewards.total_rewards_24h
        );
        println!(
            "   • Last 7 days: {} psy",
            rewards.total_rewards_7d
        );
        println!(
            "   • Last 30 days: {} psy",
            rewards.total_rewards_30d
        );

        if rewards.total_proofs > 0 {
            let claimed_percentage =
                (rewards.claimed_proofs as f64) / (rewards.total_proofs as f64) * 100.0;
            println!("   • {:.1}% of rewards are claimed", claimed_percentage);

            // Verify calculation consistency
            assert_eq!(
                rewards.total_proofs,
                rewards.claimed_proofs + rewards.unclaimed_proofs
            );
            assert_eq!(
                rewards.total_rewards,
                rewards.claimed_rewards + rewards.unclaimed_rewards
            );
            assert_eq!(
                rewards.claimed_rewards,
                rewards.claimed_proofs * 5_000_000_000
            );
            assert_eq!(
                rewards.unclaimed_rewards,
                rewards.unclaimed_proofs * 5_000_000_000
            );
            println!("   ✅ All calculations verified correct!");
        }
    }

    println!("\n🎯 Key Features Demonstrated:");
    println!("   ✓ Only COMPLETED jobs with GenerateStandardProof topic count for rewards");
    println!("   ✓ Each proof earns exactly 5×10⁹ psy");
    println!("   ✓ Claimed rewards: checkpoint_id < query parameter");
    println!("   ✓ Unclaimed rewards: checkpoint_id >= query parameter");
    println!("   ✓ Total rewards = claimed + unclaimed");
    println!("   🆕 Time-based rewards: 24h, 7d, 30d total rewards (claimed + unclaimed)");

    println!("\n🎉 Worker rewards API example completed successfully!");
    println!(
        "💡 Tip: Try different checkpoint_id values to see how claimed/unclaimed rewards change."
    );

    Ok(())
}

/// Helper function to create a sample QProvingJobDataID with GenerateStandardProof topic
fn create_generate_standard_proof_job_id(_job_name: &str) -> serde_json::Value {
    let job = QProvingJobDataID {
        topic: QJobTopic::GenerateStandardProof, // This is the key - only this topic counts for rewards
        goal_id: 0,
        circuit_type: ProvingJobCircuitType::UserEndCap,
        group_id: 0,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType::OutputProof,
        data_index: 0,
    };
    serde_json::to_value(job).unwrap()
}

/// Helper function to create a job ID with a different topic (should not count for rewards)
fn create_different_topic_job_id(_job_name: &str) -> serde_json::Value {
    let job = QProvingJobDataID {
        topic: QJobTopic::GenerateGroth16Proof, // Different topic - should NOT count for rewards
        goal_id: 0,
        circuit_type: ProvingJobCircuitType::UserEndCap,
        group_id: 0,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType::OutputProof,
        data_index: 0,
    };
    serde_json::to_value(job).unwrap()
}
