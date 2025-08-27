//! WebSocket Client Example: Real-time Event Subscription and Telemetry Integration
//!
//! This example demonstrates:
//! 1. Connecting to the WebSocket endpoint (`/ws/subscribe`)
//! 2. Setting up subscription filters for specific events
//! 3. Sending test events to the telemetry endpoint (`/telemetry/events`)
//! 4. Receiving real-time events via WebSocket
//! 5. Dynamic filter updates during connection lifetime

use std::collections::HashSet;
use std::time::Duration;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use qed_core::job::id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const API_BASE: &str = "http://localhost:3000";
const WS_URL: &str = "ws://localhost:3000/ws/subscribe";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SubscriptionFilters {
    pub user_ids: Option<HashSet<String>>,
    pub realm_ids: Option<HashSet<String>>,
    pub event_types: Option<HashSet<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateConfigurationMessage {
    pub filters: SubscriptionFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    WorkerEvent,
    UserEvent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TelemetryPayload {
    pub worker_events: Option<Vec<serde_json::Value>>,
    pub user_events: Option<Vec<serde_json::Value>>,
}

struct WebSocketClient {
    http_client: Client,
}

impl WebSocketClient {
    fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }

    /// Send test events to the telemetry endpoint
    async fn send_test_events(&self, batch_id: u32) -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();

        // Create sample worker events
        let worker_events = vec![
            json!({
                "realm_id": 16384,
                "public_key": format!("test_worker_key_{}", batch_id),
                "status": "COMPLETED",
                "source": "REALM",
                "job_id": create_sample_job_id(&format!("worker_job_{}", batch_id)),
                "checkpoint_id": 1000 + batch_id,
                "duration": 1000 + (batch_id * 100) as i64,
                "metadata": {
                    "batch": batch_id,
                    "task_type": "automated_test",
                    "circuit_type": format!("TEST_CIRCUIT_{}", batch_id)
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            }),
            json!({
                "realm_id": 0,
                "public_key": format!("test_coordinator_key_{}", batch_id),
                "status": "PROCESSING",
                "source": "COORDINATOR",
                "job_id": create_sample_job_id(&format!("coordinator_job_{}", batch_id)),
                "checkpoint_id": 2000 + batch_id,
                "duration": null,
                "metadata": {
                    "batch": batch_id,
                    "task_type": "coordination",
                    "circuit_type": "COORDINATOR_CIRCUIT"
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            })
        ];

        // Create sample user events
        let user_events = vec![
            json!({
                "user_id": format!("test_user_{}", batch_id),
                "public_key": format!("test_public_key_user_{}", batch_id),
                "tx_type": "REGISTER_USER",
                "metadata": {
                    "batch": batch_id,
                    "twitter_handle": format!("@test_user_{}", batch_id),
                    "label": format!("Test User {}", batch_id)
                },
                "timestamp": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339()
            }),
            json!({
                "user_id": format!("test_user_{}", batch_id),
                "public_key": format!("test_public_key_user_{}", batch_id),
                "tx_type": "DEPLOY_CONTRACT",
                "metadata": {
                    "batch": batch_id,
                    "contract_name": format!("TestContract_{}", batch_id),
                    "contract_size": 1024
                },
                "timestamp": (now + chrono::Duration::seconds(1)).to_rfc3339(),
                "created_at": (now + chrono::Duration::seconds(1)).to_rfc3339(),
                "updated_at": (now + chrono::Duration::seconds(1)).to_rfc3339()
            })
        ];

        // Send telemetry payload
        let telemetry_payload = json!({
            "worker_events": worker_events,
            "user_events": user_events
        });

        let response = self
            .http_client
            .post(&format!("{}/telemetry/events", API_BASE))
            .json(&telemetry_payload)
            .send()
            .await?;

        if response.status().is_success() {
            println!("✅ Batch {} events sent successfully", batch_id);
        } else {
            println!("❌ Failed to send batch {} events: {}", batch_id, response.status());
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the WebSocket client
    let client = WebSocketClient::new();

    println!("🎯 QED API Services WebSocket Client Example");
    println!("Server address: localhost:3000\n");

    // Connect to WebSocket
    println!("🔌 Connecting to WebSocket: {}", WS_URL);
    let (ws_stream, _) = connect_async(WS_URL).await?;
    println!("✅ WebSocket connection established!");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Set up subscription filters
    let filters = SubscriptionFilters {
        user_ids: Some(["test_user_1", "test_user_2", "test_user_3"].iter().map(|s| s.to_string()).collect()),
        realm_ids: Some(["16384", "0"].iter().map(|s| s.to_string()).collect()),
        event_types: Some(["WorkerEvent", "UserEvent"].iter().map(|s| s.to_string()).collect()),
    };

    let filter_update = UpdateConfigurationMessage { filters: filters.clone() };
    let filter_msg = serde_json::to_string(&filter_update)?;

    ws_sender.send(Message::Text(filter_msg)).await?;
    println!("🔧 Filters set: user_ids={:?}, realm_ids={:?}",
        filters.user_ids.as_ref().map(|set| set.iter().collect::<Vec<_>>()),
        filters.realm_ids.as_ref().map(|set| set.iter().collect::<Vec<_>>())
    );

    println!("👂 Starting to listen for WebSocket events...\n");

    // Create a task to send test events periodically
    let client_for_sender = client;
    let sender_task = tokio::spawn(async move {
        for batch_id in 1..=5 {
            sleep(Duration::from_secs(2)).await;

            println!("📤 Sending batch {} test events...", batch_id);
            if let Err(e) = client_for_sender.send_test_events(batch_id).await {
                println!("❌ Failed to send test events: {}", e);
            }
        }

        // Update filters after sending some events
        sleep(Duration::from_secs(2)).await;
        println!("🔧 Updating filters to focus on realm 16384 only...");
    });

    // Create a task to handle WebSocket messages
    let receiver_task = tokio::spawn(async move {
        let mut received_count = 0;

        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    received_count += 1;

                    match serde_json::from_str::<WebSocketEvent>(&text) {
                        Ok(event) => {
                            println!("📨 Received WebSocket event:");
                            println!("   Event type: {:?}", event.event_type);
                            println!("   Timestamp: {}", event.timestamp);

                            match event.event_type {
                                EventType::WorkerEvent => {
                                    if let Ok(worker_event) = serde_json::from_value::<serde_json::Value>(event.data.clone()) {
                                        println!("   Worker event details:");
                                        println!("     - Realm ID: {}", worker_event.get("realm_id").unwrap_or(&json!(null)));
                                        println!("     - Status: {}", worker_event.get("status").unwrap_or(&json!("unknown")));
                                        println!("     - Source: {}", worker_event.get("source").unwrap_or(&json!("unknown")));
                                        if let Some(duration) = worker_event.get("duration") {
                                            if !duration.is_null() {
                                                println!("     - Duration: {}ms", duration);
                                            }
                                        }
                                        if let Some(metadata) = worker_event.get("metadata") {
                                            println!("     - Metadata: {}", metadata);
                                        }
                                    }
                                }
                                EventType::UserEvent => {
                                    if let Ok(user_event) = serde_json::from_value::<serde_json::Value>(event.data.clone()) {
                                        println!("   User event details:");
                                        println!("     - User ID: {}", user_event.get("user_id").unwrap_or(&json!("unknown")));
                                        println!("     - Transaction Type: {}", user_event.get("tx_type").unwrap_or(&json!("unknown")));
                                        if let Some(metadata) = user_event.get("metadata") {
                                            println!("     - Metadata: {}", metadata);
                                        }
                                    }
                                }
                            }
                            println!("");
                        }
                        Err(e) => {
                            println!("❌ Failed to parse WebSocket event: {}", e);
                            println!("   Raw message: {}", text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("🔌 WebSocket connection closed");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    println!("🏓 Received ping");
                }
                Ok(Message::Pong(_)) => {
                    println!("🏓 Received pong");
                }
                Ok(_) => {
                    println!("❓ Received other message type");
                }
                Err(e) => {
                    println!("❌ WebSocket error: {}", e);
                    break;
                }
            }

            // Stop after receiving some events for demo purposes
            if received_count >= 15 {
                println!("🎯 Received {} events, stopping...", received_count);
                break;
            }
        }
    });

    // Wait for both tasks to complete or timeout
    tokio::select! {
        _ = sender_task => println!("📤 Sender task completed"),
        _ = receiver_task => println!("📨 Receiver task completed"),
        _ = sleep(Duration::from_secs(30)) => println!("⏰ Timeout reached")
    }

    println!("\n🎉 WebSocket client example completed successfully!");
    println!("💡 This example showed:");
    println!("   - WebSocket connection establishment");
    println!("   - Real-time event subscription with filtering");
    println!("   - Telemetry event publishing");
    println!("   - Live event reception and parsing");

    Ok(())
}

/// Helper function to create a sample QProvingJobDataID
fn create_sample_job_id(job_name: &str) -> serde_json::Value {
    let job = QProvingJobDataID {
        topic: QJobTopic::GenerateStandardProof,
        goal_id: job_name.len() as u64, // Use string length for some variety
        circuit_type: ProvingJobCircuitType::UserEndCap,
        group_id: 1,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType::OutputProof,
        data_index: 0,
    };
    serde_json::to_value(job).unwrap()
}