//! User Management API Example: /register and /user_info
//!
//! This example demonstrates how to use Psy API service user registration and
//! query functionality

use reqwest::Client;
use serde_json::json;

const API_BASE: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Register a user
    println!("Registering user...");
    let register_payload = json!({
        "public_key": "0x1234567890abcdef1234567890abcdef12345678",
        "twitter_handle": "@example_user",
        "label": "Psy Test User",
        "signature": "mock_signature_123456789"
    });
    println!("Payload: {}", serde_json::to_string_pretty(&register_payload)?);
    let response = client.post(&format!("{}/register", API_BASE)).json(&register_payload).send().await?;
    let body: serde_json::Value = response.json().await?;
    println!("User registration response body: {}", serde_json::to_string_pretty(&body)?);

    // Get User Info
    println!("Getting user info...");
    let public_key = "0x1234567890abcdef1234567890abcdef12345678";
    let response = client.get(&format!("{}/user_info?public_key={}", API_BASE, public_key)).send().await?;
    let body: serde_json::Value = response.json().await?;
    println!("User Info: {}", serde_json::to_string_pretty(&body)?);

    Ok(())
}
