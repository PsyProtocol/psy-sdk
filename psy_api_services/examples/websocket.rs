//! WebSocket Client Example: Real-time Event Subscription and Telemetry
//! Integration
//!
//! This example demonstrates:
//! 1. Connecting to the new specialized WebSocket endpoints (`/ws/user_event`,
//!    `/ws/worker_event`)
//! 2. Setting up subscription filters for specific events
//! 3. Sending test events to the telemetry endpoint (`/telemetry/events`)
//! 4. Receiving real-time events via WebSocket
//! 5. Dynamic filter updates during connection lifetime

use std::time::Duration;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use psy_core::job::id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const API_BASE: &str = "http://localhost:3000";
const USER_EVENT_WS_URL: &str = "ws://localhost:3000/ws/user_event";
const WORKER_EVENT_WS_URL: &str = "ws://localhost:3000/ws/worker_event";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the WebSocket client
    let client = WebSocketClient::new();

    println!("🎯 QED API Services WebSocket Client Example (New Specialized Endpoints)");
    println!("Server address: localhost:3000\n");

    // Connect to both WebSocket endpoints
    println!("🔌 Connecting to User Event WebSocket: {}", USER_EVENT_WS_URL);
    let (user_ws_stream, _) = connect_async(USER_EVENT_WS_URL).await?;
    println!("✅ User Event WebSocket connection established!");

    println!("🔌 Connecting to Worker Event WebSocket: {}", WORKER_EVENT_WS_URL);
    let (worker_ws_stream, _) = connect_async(WORKER_EVENT_WS_URL).await?;
    println!("✅ Worker Event WebSocket connection established!");

    let (mut user_ws_sender, mut user_ws_receiver) = user_ws_stream.split();
    let (mut worker_ws_sender, mut worker_ws_receiver) = worker_ws_stream.split();

    // Set up subscription filters for user events
    let user_filters = UserEventFilters {
        user_id: Some("test_user_1".to_string()),
    };

    let user_filter_update = UpdateUserEventConfigurationMessage {
        filters: user_filters.clone(),
    };
    let user_filter_msg = serde_json::to_string(&user_filter_update)?;

    user_ws_sender.send(Message::Text(user_filter_msg.into())).await?;
    println!("🔧 User Event filters set: user_id={:?}", user_filters.user_id);

    // Set up subscription filters for worker events
    let worker_filters = WorkerEventFilters {
        realm_id: Some("16384".to_string()),
        worker_pubkey: Some(vec!["test_worker_key_1".to_string(), "test_worker_key_2".to_string()]),
    };

    let worker_filter_update = UpdateWorkerEventConfigurationMessage {
        filters: worker_filters.clone(),
    };
    let worker_filter_msg = serde_json::to_string(&worker_filter_update)?;

    worker_ws_sender.send(Message::Text(worker_filter_msg.into())).await?;
    println!(
        "🔧 Worker Event filters set: realm_id={:?}, worker_pubkey={:?}",
        worker_filters.realm_id, worker_filters.worker_pubkey
    );

    println!("👂 Starting to listen for WebSocket events from both endpoints...\n");

    // Create a task to send test events periodically
    let client_for_sender = client;
    let sender_task = tokio::spawn(async move {
        for batch_id in 1..=3 {
            sleep(Duration::from_secs(2)).await;

            println!("📤 Sending batch {} test events...", batch_id);
            if let Err(e) = client_for_sender.send_test_events(batch_id).await {
                println!("❌ Failed to send test events: {}", e);
            }
        }
    });

    // Create a task to handle User Event WebSocket messages
    let user_receiver_task = tokio::spawn(async move {
        let mut received_count = 0;

        while let Some(message) = user_ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    received_count += 1;

                    match serde_json::from_str::<WebSocketEvent>(&text) {
                        Ok(event) => {
                            println!("📨 [USER] Received User Event:");
                            println!("   Event type: {:?}", event.event_type);
                            println!("   Timestamp: {}", event.timestamp);

                            if let Ok(user_event) = serde_json::from_value::<serde_json::Value>(event.data.clone()) {
                                println!("   User event details:");
                                println!("     - User ID: {}", user_event.get("user_id").unwrap_or(&json!("unknown")));
                                println!("     - Transaction Type: {}", user_event.get("tx_type").unwrap_or(&json!("unknown")));
                                if let Some(metadata) = user_event.get("metadata") {
                                    println!("     - Metadata: {}", metadata);
                                }
                            }
                            println!("");
                        }
                        Err(e) => {
                            println!("❌ [USER] Failed to parse WebSocket event: {}", e);
                            println!("   Raw message: {}", text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("🔌 [USER] WebSocket connection closed");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    println!("🏓 [USER] Received ping");
                }
                Ok(Message::Pong(_)) => {
                    println!("🏓 [USER] Received pong");
                }
                Ok(_) => {
                    println!("❓ [USER] Received other message type");
                }
                Err(e) => {
                    println!("❌ [USER] WebSocket error: {}", e);
                    break;
                }
            }

            // Stop after receiving some events for demo purposes
            if received_count >= 8 {
                println!("🎯 [USER] Received {} events, stopping...", received_count);
                break;
            }
        }
    });

    // Create a task to handle Worker Event WebSocket messages
    let worker_receiver_task = tokio::spawn(async move {
        let mut received_count = 0;

        while let Some(message) = worker_ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    received_count += 1;

                    match serde_json::from_str::<WebSocketEvent>(&text) {
                        Ok(event) => {
                            println!("📨 [WORKER] Received Worker Event:");
                            println!("   Event type: {:?}", event.event_type);
                            println!("   Timestamp: {}", event.timestamp);

                            if let Ok(worker_event) = serde_json::from_value::<serde_json::Value>(event.data.clone()) {
                                println!("   Worker event details:");
                                println!("     - Realm ID: {}", worker_event.get("realm_id").unwrap_or(&json!(null)));
                                println!("     - Status: {}", worker_event.get("status").unwrap_or(&json!("unknown")));
                                println!("     - Source: {}", worker_event.get("source").unwrap_or(&json!("unknown")));
                                if let Some(public_key) = worker_event.get("public_key") {
                                    println!("     - Public Key: {}", public_key);
                                }
                                if let Some(duration) = worker_event.get("duration") {
                                    if !duration.is_null() {
                                        println!("     - Duration: {}ms", duration);
                                    }
                                }
                                if let Some(metadata) = worker_event.get("metadata") {
                                    println!("     - Metadata: {}", metadata);
                                }
                            }
                            println!("");
                        }
                        Err(e) => {
                            println!("❌ [WORKER] Failed to parse WebSocket event: {}", e);
                            println!("   Raw message: {}", text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("🔌 [WORKER] WebSocket connection closed");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    println!("🏓 [WORKER] Received ping");
                }
                Ok(Message::Pong(_)) => {
                    println!("🏓 [WORKER] Received pong");
                }
                Ok(_) => {
                    println!("❓ [WORKER] Received other message type");
                }
                Err(e) => {
                    println!("❌ [WORKER] WebSocket error: {}", e);
                    break;
                }
            }

            // Stop after receiving some events for demo purposes
            if received_count >= 8 {
                println!("🎯 [WORKER] Received {} events, stopping...", received_count);
                break;
            }
        }
    });

    // Wait for all tasks to complete or timeout
    tokio::select! {
        _ = sender_task => println!("📤 Sender task completed"),
        _ = user_receiver_task => println!("📨 User receiver task completed"),
        _ = worker_receiver_task => println!("📨 Worker receiver task completed"),
        _ = sleep(Duration::from_secs(45)) => println!("⏰ Timeout reached")
    }

    println!("\n🎉 WebSocket client example completed successfully!");
    println!("💡 This example showed:");
    println!("   - Connection to specialized WebSocket endpoints:");
    println!("     • /ws/user_event - for user events with user_id filtering");
    println!("     • /ws/worker_event - for worker events with realm_id and worker_pubkey filtering");
    println!("   - Real-time event subscription with specialized filtering");
    println!("   - Telemetry event publishing");
    println!("   - Live event reception and parsing from both endpoints");
    println!("   - Dedicated filtering for each event type");

    Ok(())
}

/// Helper function to create a sample QProvingJobDataID
fn create_sample_job_id(job_name: &str) -> serde_json::Value {
    let job = QProvingJobDataID {
        topic: QJobTopic::GenerateStandardProof,
        goal_id: job_name.len() as u64, // Use string length for some variety
        slot_id: 0,
        circuit_type: ProvingJobCircuitType::UserEndCap,
        group_id: 1,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType::OutputProof,
        data_index: 0,
    };
    serde_json::to_value(job).unwrap()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UserEventFilters {
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WorkerEventFilters {
    pub realm_id: Option<String>,
    pub worker_pubkey: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUserEventConfigurationMessage {
    pub filters: UserEventFilters,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateWorkerEventConfigurationMessage {
    pub filters: WorkerEventFilters,
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
        Self { http_client: Client::new() }
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
            }),
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
            }),
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
