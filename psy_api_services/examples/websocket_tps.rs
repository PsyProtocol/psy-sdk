//! TPS WebSocket Client Example: Real-time Transaction Per Second (TPS)
//! Monitoring
//!
//! This example demonstrates:
//! 1. Connecting to the TPS WebSocket endpoint (`/ws/tps`)
//! 2. Receiving real-time TPS updates every 12 seconds
//! 3. Creating test user events with different transaction counts
//! 4. Monitoring how TPS calculations adapt to different GUTA transaction
//!    counts
//! 5. Understanding the dynamic transaction counting mechanism

use std::time::Duration;

use chrono::Utc;
use futures_util::StreamExt;
// Import types from the API service crate instead of redefining them
use psy_api_services::handlers::websocket::{EventType, WebSocketEvent};
use psy_api_services::models::TpsData;
use reqwest::Client;
use serde_json::json;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const API_BASE: &str = "http://localhost:3000";
const TPS_WS_URL: &str = "ws://localhost:3000/ws/tps";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TpsTestClient::new();

    println!("📊 Psy API Services TPS WebSocket Client Example");
    println!("Server address: localhost:3000\n");

    // Connect to TPS WebSocket
    println!("🔌 Connecting to TPS WebSocket: {}", TPS_WS_URL);
    let (ws_stream, _) = connect_async(TPS_WS_URL).await?;
    println!("✅ TPS WebSocket connection established!");

    let (_, mut ws_receiver) = ws_stream.split();

    println!("📡 Starting TPS monitoring (updates every 12 seconds)...\n");

    // Statistics tracking
    let mut tps_history: Vec<TpsData> = Vec::new();
    let mut max_tps = 0.0f64;
    let mut total_transactions_observed = 0i64;

    // Create a task to send test events at strategic intervals
    let client_for_sender = client;
    let sender_task = tokio::spawn(async move {
        // Wait a bit before starting to send events
        sleep(Duration::from_secs(2)).await;

        println!("🚀 Starting test event generation...\n");

        for batch_id in 1..=3 {
            println!("📤 Generating test batch {} with diverse transaction patterns...", batch_id);
            if let Err(e) = client_for_sender.send_test_user_events(batch_id).await {
                println!("❌ Failed to send test events: {}", e);
            }

            // Wait between batches to see TPS changes
            sleep(Duration::from_secs(6)).await;
        }

        println!("\n⏳ Test event generation completed. Continuing to monitor TPS updates...");
    });

    // Create a task to handle TPS WebSocket messages
    let receiver_task = tokio::spawn(async move {
        let mut update_count = 0;

        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    update_count += 1;

                    match serde_json::from_str::<WebSocketEvent>(&text) {
                        Ok(event) => {
                            if matches!(event.event_type, EventType::TpsUpdate) {
                                match serde_json::from_value::<TpsData>(event.data.clone()) {
                                    Ok(tps_data) => {
                                        println!("📊 TPS Update #{}", update_count);
                                        println!("   🔥 Current TPS: {:.3} transactions/second", tps_data.tps);
                                        println!(
                                            "   📈 Transaction Count: {} transactions in {}s window",
                                            tps_data.transaction_count, tps_data.time_window_seconds
                                        );
                                        println!("   ⏰ Timestamp: {}", tps_data.timestamp.format("%H:%M:%S"));

                                        // Update statistics
                                        if tps_data.tps > max_tps {
                                            max_tps = tps_data.tps;
                                            println!("   🎯 New TPS record: {:.3}!", max_tps);
                                        }

                                        total_transactions_observed += tps_data.transaction_count;

                                        // Show trend if we have enough data points
                                        let change_info = if tps_history.len() >= 1 {
                                            let previous = &tps_history[tps_history.len() - 1];
                                            let change = tps_data.tps - previous.tps;
                                            Some(change)
                                        } else {
                                            None
                                        };

                                        tps_history.push(tps_data);

                                        if let Some(change) = change_info {
                                            let trend = if change > 0.001 {
                                                format!("📈 +{:.3}", change)
                                            } else if change < -0.001 {
                                                format!("📉 {:.3}", change)
                                            } else {
                                                "➡️ stable".to_string()
                                            };
                                            println!("   📊 Trend: {} (vs previous)", trend);
                                        }

                                        println!("   💡 This TPS reflects dynamic transaction counting:");
                                        println!("      - RegisterUser/DeployContract = 1 tx each");
                                        println!("      - GUTA = metadata.transaction_count (or default 2)");
                                        println!("");
                                    }
                                    Err(e) => {
                                        println!("❌ Failed to parse TPS data: {}", e);
                                        println!("   Raw data: {}", event.data);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to parse WebSocket event: {}", e);
                            println!("   Raw message: {}", text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("🔌 TPS WebSocket connection closed");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    println!("🏓 Received ping from TPS WebSocket");
                }
                Ok(Message::Pong(_)) => {
                    println!("🏓 Received pong from TPS WebSocket");
                }
                Ok(_) => {
                    println!("❓ Received other message type from TPS WebSocket");
                }
                Err(e) => {
                    println!("❌ TPS WebSocket error: {}", e);
                    break;
                }
            }

            // Stop after receiving several updates for demo purposes
            if update_count >= 8 {
                println!("📊 Received {} TPS updates, stopping monitoring...", update_count);
                break;
            }
        }

        // Display summary statistics
        println!("\n📈 TPS Monitoring Summary:");
        println!("   🏆 Peak TPS: {:.3} transactions/second", max_tps);
        println!("   📊 Total Updates Received: {}", update_count);
        println!("   🔢 Total Transactions Observed: {}", total_transactions_observed);

        if !tps_history.is_empty() {
            let avg_tps: f64 = tps_history.iter().map(|t| t.tps).sum::<f64>() / tps_history.len() as f64;
            println!("   📊 Average TPS: {:.3} transactions/second", avg_tps);

            println!("\n📊 TPS History:");
            for (i, tps_data) in tps_history.iter().enumerate() {
                println!(
                    "   {}. {}: {:.3} TPS ({} transactions)",
                    i + 1,
                    tps_data.timestamp.format("%H:%M:%S"),
                    tps_data.tps,
                    tps_data.transaction_count
                );
            }
        }
    });

    // Wait for both tasks to complete or timeout
    tokio::select! {
        _ = sender_task => println!("📤 Test event sender completed"),
        _ = receiver_task => println!("📨 TPS receiver completed"),
        _ = sleep(Duration::from_secs(120)) => println!("⏰ Demo timeout reached (2 minutes)")
    }

    println!("\n🎉 TPS WebSocket client example completed successfully!");
    println!("\n💡 This example demonstrated:");
    println!("   ✅ Real-time TPS monitoring via WebSocket (/ws/tps)");
    println!("   ✅ Dynamic transaction counting based on user event metadata");
    println!("   ✅ Support for multiple transaction count field names:");
    println!("      - metadata.transaction_count");
    println!("      - metadata.transactions");
    println!("      - metadata.tx_count");
    println!("   ✅ Fallback to default values when metadata is missing");
    println!("   ✅ 12-second rolling window TPS calculation");
    println!("   ✅ Extensible design for future GUTA transaction types");

    Ok(())
}

struct TpsTestClient {
    http_client: Client,
}

impl TpsTestClient {
    fn new() -> Self {
        Self { http_client: Client::new() }
    }

    /// Send test user events with varying transaction counts to demonstrate
    /// dynamic TPS calculation
    async fn send_test_user_events(&self, batch_id: u32) -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();

        // Create diverse user events to showcase different transaction counting
        // scenarios
        let mut user_events = Vec::new();

        // Standard RegisterUser events (1 transaction each)
        for i in 0..2 {
            user_events.push(json!({
                "user_id": format!("reg_user_{}_{}", batch_id, i),
                "public_key": format!("reg_key_{}_{}", batch_id, i),
                "tx_type": "REGISTER_USER",
                "metadata": {
                    "batch": batch_id,
                    "registration_type": "standard",
                    "user_type": "individual"
                },
                "timestamp": (now + chrono::Duration::milliseconds(i * 100)).to_rfc3339(),
                "created_at": (now + chrono::Duration::milliseconds(i * 100)).to_rfc3339(),
                "updated_at": (now + chrono::Duration::milliseconds(i * 100)).to_rfc3339()
            }));
        }

        // Standard DeployContract events (1 transaction each)
        for i in 0..1 {
            user_events.push(json!({
                "user_id": format!("contract_user_{}_{}", batch_id, i),
                "public_key": format!("contract_key_{}_{}", batch_id, i),
                "tx_type": "DEPLOY_CONTRACT",
                "metadata": {
                    "batch": batch_id,
                    "contract_name": format!("MyContract_{}_{}", batch_id, i),
                    "contract_version": "1.0.0"
                },
                "timestamp": (now + chrono::Duration::milliseconds(300 + i * 100)).to_rfc3339(),
                "created_at": (now + chrono::Duration::milliseconds(300 + i * 100)).to_rfc3339(),
                "updated_at": (now + chrono::Duration::milliseconds(300 + i * 100)).to_rfc3339()
            }));
        }

        // GUTA events with default transaction count (2 transactions - fallback
        // behavior)
        user_events.push(json!({
            "user_id": format!("guta_user_default_{}", batch_id),
            "public_key": format!("guta_key_default_{}", batch_id),
            "tx_type": "GUTA",
            "metadata": {
                "batch": batch_id,
                "guta_type": "standard_batch",
                "description": "Standard GUTA without explicit transaction_count (uses default: 2)"
            },
            "timestamp": (now + chrono::Duration::milliseconds(500)).to_rfc3339(),
            "created_at": (now + chrono::Duration::milliseconds(500)).to_rfc3339(),
            "updated_at": (now + chrono::Duration::milliseconds(500)).to_rfc3339()
        }));

        // GUTA events with explicit transaction_count (demonstrating dynamic counting)
        let guta_scenarios = vec![("small_batch", 3), ("medium_batch", 7), ("large_batch", 15)];

        for (idx, (guta_type, tx_count)) in guta_scenarios.into_iter().enumerate() {
            user_events.push(json!({
                "user_id": format!("guta_user_{}_{}", guta_type, batch_id),
                "public_key": format!("guta_key_{}_{}", guta_type, batch_id),
                "tx_type": "GUTA",
                "metadata": {
                    "batch": batch_id,
                    "guta_type": guta_type,
                    "transaction_count": tx_count,  // 🎯 This field enables dynamic transaction counting
                    "description": format!("GUTA with {} transactions", tx_count)
                },
                "timestamp": (now + chrono::Duration::milliseconds(700 + idx as i64 * 100)).to_rfc3339(),
                "created_at": (now + chrono::Duration::milliseconds(700 + idx as i64 * 100)).to_rfc3339(),
                "updated_at": (now + chrono::Duration::milliseconds(700 + idx as i64 * 100)).to_rfc3339()
            }));
        }

        // Additional GUTA with alternative metadata field names (testing flexibility)
        user_events.push(json!({
            "user_id": format!("guta_user_flexible_{}", batch_id),
            "public_key": format!("guta_key_flexible_{}", batch_id),
            "tx_type": "GUTA",
            "metadata": {
                "batch": batch_id,
                "guta_type": "flexible_naming",
                "transactions": 5,  // 🎯 Alternative field name: "transactions"
                "description": "GUTA using 'transactions' field name"
            },
            "timestamp": (now + chrono::Duration::seconds(1)).to_rfc3339(),
            "created_at": (now + chrono::Duration::seconds(1)).to_rfc3339(),
            "updated_at": (now + chrono::Duration::seconds(1)).to_rfc3339()
        }));

        user_events.push(json!({
            "user_id": format!("guta_user_alt_{}", batch_id),
            "public_key": format!("guta_key_alt_{}", batch_id),
            "tx_type": "GUTA",
            "metadata": {
                "batch": batch_id,
                "guta_type": "alternative_naming",
                "tx_count": 12,  // 🎯 Alternative field name: "tx_count"
                "description": "GUTA using 'tx_count' field name"
            },
            "timestamp": (now + chrono::Duration::seconds(1) + chrono::Duration::milliseconds(200)).to_rfc3339(),
            "created_at": (now + chrono::Duration::seconds(1) + chrono::Duration::milliseconds(200)).to_rfc3339(),
            "updated_at": (now + chrono::Duration::seconds(1) + chrono::Duration::milliseconds(200)).to_rfc3339()
        }));

        // Calculate expected transaction count for this batch
        let expected_transactions = 2 + 1 + 2 + 3 + 7 + 15 + 5 + 12; // 47 total
        println!("🧮 Expected transactions in batch {}: {} transactions", batch_id, expected_transactions);

        // Send telemetry payload
        let telemetry_payload = json!({
            "user_events": user_events
        });

        let response = self
            .http_client
            .post(&format!("{}/telemetry/events", API_BASE))
            .json(&telemetry_payload)
            .send()
            .await?;

        if response.status().is_success() {
            println!("✅ Batch {} user events sent successfully", batch_id);
            println!("   - 2 RegisterUser events (1 tx each) = 2 transactions");
            println!("   - 1 DeployContract event (1 tx each) = 1 transaction");
            println!("   - 1 GUTA with default count = 2 transactions");
            println!("   - 1 GUTA with transaction_count=3 = 3 transactions");
            println!("   - 1 GUTA with transaction_count=7 = 7 transactions");
            println!("   - 1 GUTA with transaction_count=15 = 15 transactions");
            println!("   - 1 GUTA with transactions=5 = 5 transactions");
            println!("   - 1 GUTA with tx_count=12 = 12 transactions");
            println!("   📊 Total: {} transactions", expected_transactions);
        } else {
            println!("❌ Failed to send batch {} user events: {}", batch_id, response.status());
        }

        Ok(())
    }
}
