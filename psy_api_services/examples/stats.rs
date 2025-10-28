//! Statistics API Example: /stats related endpoints
//!
//! This example demonstrates how to use QED API service statistics functionality including global stats, realm stats and worker stats

use reqwest::Client;
use std::collections::HashMap;

const API_BASE: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    println!("📊 QED Getting /stats");
    let response = client.get(&format!("{}/stats", API_BASE)).send().await?;
    let body: serde_json::Value = response.json().await?;
    println!("✅ System statistics retrieved successfully:");
    println!("{}", serde_json::to_string_pretty(&body)?);

    println!("📊 QED Getting /stats/realms");
    let response = client
        .get(&format!("{}/stats/realms", API_BASE))
        .send()
        .await?;
    let body: serde_json::Value = response.json().await?;
    println!("✅ Global realm statistics retrieved successfully:");
    println!("{}", serde_json::to_string_pretty(&body)?);

    println!("📊 QED Getting /stats/realms/{{realm_id}}");
    let realm_id = 16384;
    let response = client
        .get(&format!("{}/stats/realms/{}", API_BASE, realm_id))
        .send()
        .await?;
    let body: serde_json::Value = response.json().await?;
    println!("✅ Realm {} statistics retrieved successfully:", realm_id);
    println!("{}", serde_json::to_string_pretty(&body)?);

    println!("📊 QED Getting /stats/workers/{}", realm_id);
    let worker_id = "0x1234567890abcdef1234567890abcdef12345678";
    let response = client
        .get(&format!("{}/stats/workers/{}", API_BASE, worker_id))
        .send()
        .await?;
    let body: serde_json::Value = response.json().await?;
    println!("✅ Worker {} statistics retrieved successfully:", worker_id);
    println!("{}", serde_json::to_string_pretty(&body)?);

    Ok(())
}
