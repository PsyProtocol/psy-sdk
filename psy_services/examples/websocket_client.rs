// examples/websocket_client.rs
//! WebSocket client example for testing the unified WebSocket endpoint
//!
//! This example demonstrates:
//! - Connecting to the unified WebSocket endpoint
//! - Subscribing to multiple channels
//! - Updating filters dynamically
//! - Ping-pong heartbeat mechanism
//! - Handling different message types
//!
//! Usage:
//!   cargo run --example websocket_client
//!   cargo run --example websocket_client -- --url ws://localhost:3000/ws
//!   cargo run --example websocket_client -- --channels
//! worker_events,user_events,tps

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, Utf8Bytes},
};
// ============================================================================
// Message Types (must match server)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
    UpdateFilters { filters: Filters },
    Ping, // Client sends ping to server
    Pong, // Client responds to server ping
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Event {
        channel: String,
        data: serde_json::Value,
        timestamp: String,
    },
    Subscribed {
        channels: Vec<String>,
    },
    Unsubscribed {
        channels: Vec<String>,
    },
    Error {
        code: String,
        message: String,
    },
    Ping, // Server sends ping to client
    Pong, // Server responds to client ping
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Filters {
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_realm_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_type: Option<String>,
}

// ============================================================================
// CLI Arguments
// ============================================================================

#[derive(Parser, Debug)]
#[command(name = "websocket_client")]
#[command(about = "WebSocket client for testing unified WebSocket endpoint")]
struct Args {
    /// WebSocket URL to connect to
    #[arg(long, default_value = "ws://localhost:3000/ws")]
    url: String,

    /// Comma-separated list of channels to subscribe to on connect
    /// Options: worker_events, user_events, tps
    #[arg(long, default_value = "worker_events,user_events,tps")]
    channels: String,

    /// Filter by worker realm ID
    #[arg(long)]
    worker_realm_id: Option<i64>,

    /// Filter by worker public key
    #[arg(long)]
    worker_public_key: Option<String>,

    /// Filter by worker status (e.g., COMPLETED, FAILED)
    #[arg(long)]
    worker_status: Option<String>,

    /// Filter by user ID
    #[arg(long)]
    user_id: Option<String>,

    /// Enable ping-pong heartbeat (sends ping every N seconds)
    #[arg(long, default_value = "30")]
    ping_interval: u64,

    /// Test mode: performs various subscription/filter operations
    #[arg(long)]
    test_mode: bool,
}

// ============================================================================
// Statistics
// ============================================================================

#[derive(Default)]
struct Statistics {
    worker_events_received: u64,
    user_events_received: u64,
    tps_updates_received: u64,
    pings_sent: u64,
    pongs_received: u64,
    server_pings_received: u64,
    pongs_sent: u64,
    errors_received: u64,
}

impl Statistics {
    fn print(&self) {
        println!("\n{}", "=== Statistics ===".cyan().bold());
        println!("Worker Events: {}", self.worker_events_received.to_string().green());
        println!("User Events: {}", self.user_events_received.to_string().green());
        println!("TPS Updates: {}", self.tps_updates_received.to_string().green());
        println!("{}", "--- Heartbeat ---".cyan());
        println!("Client Pings Sent: {}", self.pings_sent.to_string().yellow());
        println!("Client Pongs Received: {}", self.pongs_received.to_string().yellow());
        println!("Server Pings Received: {}", self.server_pings_received.to_string().yellow());
        println!("Server Pongs Sent: {}", self.pongs_sent.to_string().yellow());
        println!("Errors: {}", self.errors_received.to_string().red());
        println!("{}", "==================".cyan().bold());
    }
}

// ============================================================================
// Main Function
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    println!("{}", "╔══════════════════════════════════════════════════════════╗".cyan().bold());
    println!("{}", "║     Psy API WebSocket Client - Testing Tool              ║".cyan().bold());
    println!("{}", "╚══════════════════════════════════════════════════════════╝".cyan().bold());
    println!();

    // Build URL with query parameters
    let channels_param = args.channels.replace(',', ",");
    let mut url = format!("{}?channels={}", args.url, channels_param);

    if let Some(realm_id) = args.worker_realm_id {
        url.push_str(&format!("&worker_realm_id={}", realm_id));
    }
    if let Some(ref public_key) = args.worker_public_key {
        url.push_str(&format!("&worker_public_key={}", public_key));
    }
    if let Some(ref user_id) = args.user_id {
        url.push_str(&format!("&user_id={}", user_id));
    }

    println!("{} {}", "Connecting to:".yellow().bold(), url.bright_white());
    println!("{} {}", "Initial channels:".yellow().bold(), args.channels.bright_white());
    if args.test_mode {
        println!("{}", "Test mode: ENABLED".green().bold());
    }
    println!();

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&url).await?;
    println!("{}", "✓ Connected successfully!".green().bold());

    let (mut write, mut read) = ws_stream.split();
    let mut stats = Statistics::default();

    // Create ping interval
    let mut ping_timer = interval(Duration::from_secs(args.ping_interval));
    ping_timer.tick().await; // Skip first immediate tick

    // Create test mode timer (if enabled)
    let mut test_timer = if args.test_mode { Some(interval(Duration::from_secs(10))) } else { None };
    if let Some(ref mut timer) = test_timer {
        timer.tick().await; // Skip first immediate tick
    }

    let mut test_step = 0;

    // Main event loop
    loop {
        tokio::select! {
            // Handle incoming messages
            Some(msg) = read.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Check if this is a server ping before parsing
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                            if matches!(server_msg, ServerMessage::Ping) {
                                // Respond to server ping immediately
                                let pong_msg = ClientMessage::Pong;
                                let json = serde_json::to_string(&pong_msg)?;
                                write.send(Message::Text(Utf8Bytes::from(json))).await?;
                                stats.pongs_sent += 1;
                                println!("{} {} (total: {})",
                                    "→".cyan(),
                                    "Sent PONG response".cyan().bold(),
                                    stats.pongs_sent.to_string().bright_white()
                                );
                            }
                            // Handle all messages including the ping we just saw
                            handle_message(&text, &mut stats)?;
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        println!("{} {:?}", "Connection closed:".red().bold(), frame);
                        break;
                    }
                    Ok(Message::Ping(_)) => {
                        println!("{}", "Received ping from server".yellow());
                    }
                    Ok(Message::Pong(_)) => {
                        println!("{}", "Received pong from server".yellow());
                    }
                    Err(e) => {
                        eprintln!("{} {}", "WebSocket error:".red().bold(), e);
                        break;
                    }
                    _ => {}
                }
            }

            // Send periodic pings
            _ = ping_timer.tick() => {
                let ping_msg = ClientMessage::Ping;
                let json = serde_json::to_string(&ping_msg)?;
                write.send(Message::Text(Utf8Bytes::from(json))).await?;
                stats.pings_sent += 1;
                println!("{} (total: {})",
                    "→ Sent PING".yellow().bold(),
                    stats.pings_sent.to_string().bright_white()
                );
            }

            // Test mode operations
            _ = async {
                match test_timer.as_mut() {
                    Some(timer) => timer.tick().await,
                    None => std::future::pending().await,
                }
            } => {
                test_step += 1;
                match test_step {
                    1 => {
                        println!("\n{}", "=== Test Step 1: Unsubscribe from TPS ===".magenta().bold());
                        let msg = ClientMessage::Unsubscribe {
                            channels: vec!["tps".to_string()],
                        };
                        write.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&msg)?))).await?;
                    }
                    2 => {
                        println!("\n{}", "=== Test Step 2: Update filters (worker realm 1 only) ===".magenta().bold());
                        let msg = ClientMessage::UpdateFilters {
                            filters: Filters {
                                worker_realm_id: Some(1),
                                worker_status: Some("COMPLETED".to_string()),
                                ..Default::default()
                            },
                        };
                        write.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&msg)?))).await?;
                    }
                    3 => {
                        println!("\n{}", "=== Test Step 3: Re-subscribe to TPS ===".magenta().bold());
                        let msg = ClientMessage::Subscribe {
                            channels: vec!["tps".to_string()],
                        };
                        write.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&msg)?))).await?;
                    }
                    4 => {
                        println!("\n{}", "=== Test Step 4: Clear filters ===".magenta().bold());
                        let msg = ClientMessage::UpdateFilters {
                            filters: Filters::default(),
                        };
                        write.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&msg)?))).await?;
                    }
                    5 => {
                        println!("\n{}", "=== Test Step 5: Unsubscribe from all ===".magenta().bold());
                        let msg = ClientMessage::Unsubscribe {
                            channels: vec![
                                "worker_events".to_string(),
                                "user_events".to_string(),
                                "tps".to_string(),
                            ],
                        };
                        write.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&msg)?))).await?;
                    }
                    6 => {
                        println!("\n{}", "=== Test Step 6: Re-subscribe to all ===".magenta().bold());
                        let msg = ClientMessage::Subscribe {
                            channels: vec![
                                "worker_events".to_string(),
                                "user_events".to_string(),
                                "tps".to_string(),
                            ],
                        };
                        write.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&msg)?))).await?;
                        test_step = 0; // Reset for next cycle
                    }
                    _ => {}
                }
            }

            // Handle Ctrl+C
            _ = tokio::signal::ctrl_c() => {
                println!("\n{}", "Received Ctrl+C, closing connection...".yellow().bold());
                stats.print();
                break;
            }
        }
    }

    println!("\n{}", "Connection closed.".yellow().bold());
    stats.print();

    Ok(())
}

// ============================================================================
// Message Handler
// ============================================================================

fn handle_message(text: &str, stats: &mut Statistics) -> Result<()> {
    let msg: ServerMessage = serde_json::from_str(text)?;

    match msg {
        ServerMessage::Event { channel, data, timestamp } => {
            match channel.as_str() {
                "worker_events" => {
                    stats.worker_events_received += 1;
                    println!(
                        "{} {} {}",
                        "←".green(),
                        "[WORKER EVENT]".green().bold(),
                        format!("(total: {})", stats.worker_events_received).bright_black()
                    );
                    if let Some(status) = data.get("status") {
                        println!("  Status: {}", status.as_str().unwrap_or("unknown").cyan());
                    }
                    if let Some(job_id) = data.get("job_id") {
                        println!("  Job ID: {}", serde_json::to_string_pretty(job_id)?);
                    }
                    if let Some(checkpoint) = data.get("checkpoint_id") {
                        println!("  Checkpoint: {}", checkpoint);
                    }
                }
                "user_events" => {
                    stats.user_events_received += 1;
                    println!(
                        "{} {} {}",
                        "←".blue(),
                        "[USER EVENT]".blue().bold(),
                        format!("(total: {})", stats.user_events_received).bright_black()
                    );
                    if let Some(user_id) = data.get("user_id") {
                        println!("  User ID: {}", user_id.as_str().unwrap_or("unknown").cyan());
                    }
                    if let Some(tx_type) = data.get("tx_type") {
                        println!("  TX Type: {}", tx_type.as_str().unwrap_or("unknown").cyan());
                    }
                }
                "tps" => {
                    stats.tps_updates_received += 1;
                    println!(
                        "{} {} {}",
                        "←".magenta(),
                        "[TPS UPDATE]".magenta().bold(),
                        format!("(total: {})", stats.tps_updates_received).bright_black()
                    );
                    if let Some(tps) = data.get("tps") {
                        println!("  TPS: {}", format!("{:.2}", tps.as_f64().unwrap_or(0.0)).bright_white());
                    }
                    if let Some(tx_count) = data.get("transaction_count") {
                        println!("  Transactions: {}", tx_count);
                    }
                    if let Some(block_height) = data.get("block_height") {
                        println!("  Block Height: {}", block_height);
                    }
                }
                _ => {
                    println!("{} Unknown channel: {}", "←".yellow(), channel);
                }
            }
            println!("  Timestamp: {}", timestamp.bright_black());
        }

        ServerMessage::Subscribed { channels } => {
            println!("{} {} {:?}", "✓".green().bold(), "Subscribed to:".green().bold(), channels);
        }

        ServerMessage::Unsubscribed { channels } => {
            println!("{} {} {:?}", "✓".yellow().bold(), "Unsubscribed from:".yellow().bold(), channels);
        }

        ServerMessage::Error { code, message } => {
            stats.errors_received += 1;
            eprintln!("{} {} [{}] {}", "✗".red().bold(), "Error:".red().bold(), code.red(), message);
        }

        ServerMessage::Ping => {
            stats.server_pings_received += 1;
            println!(
                "{} {} (total: {})",
                "←".cyan(),
                "Received server PING".cyan().bold(),
                stats.server_pings_received.to_string().bright_white()
            );
            // Note: Pong response is handled in the main loop
        }

        ServerMessage::Pong => {
            stats.pongs_received += 1;
            println!(
                "{} {} (total: {})",
                "←".yellow(),
                "Received PONG".yellow().bold(),
                stats.pongs_received.to_string().bright_white()
            );
        }
    }

    Ok(())
}
