use qed_node::worker::run_worker;
use qed_core::data::qhashout::QHashOut;
use qed_core::config::network_constants::get_default_worker_public_key;
use plonky2::field::goldilocks_field::GoldilocksField;
use tracing::info;
use std::str::FromStr;
use std::fs;

pub async fn run(config_path: String, public_key: Option<String>) -> anyhow::Result<()> {
    info!("Worker starting...");
    info!("Loading config from: {}", config_path);

    // Load edge URLs from config
    let edge_urls = load_edge_urls_from_config(&config_path)?;
    info!("Loaded edge URLs: {:?}", edge_urls);

    // Parse public key if provided
    let worker_public_key = if let Some(key_str) = public_key {
        QHashOut::<GoldilocksField>::from_str(&key_str)
            .map_err(|e| anyhow::format_err!("Failed to parse public key: {}", e))?
    } else {
        get_default_worker_public_key::<GoldilocksField>()
    };

    info!("Using worker public key: {:?}", worker_public_key);
    run_worker(edge_urls, worker_public_key).await?;
    let _ = tokio::signal::ctrl_c().await;
    info!("Worker exit.");
    Ok(())
}

fn load_edge_urls_from_config(config_path: &str) -> anyhow::Result<Vec<String>> {
    let config_str = fs::read_to_string(config_path)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;

    let mut edge_urls = Vec::new();

    // Extract coordinator URLs
    if let Some(coordinator_configs) = json_value["network"]["coordinator_configs"].as_array() {
        for config in coordinator_configs {
            if let Some(rpc_urls) = config["rpc_url"].as_array() {
                for url in rpc_urls {
                    if let Some(url_str) = url.as_str() {
                        edge_urls.push(url_str.to_string());
                    }
                }
            }
        }
    }

    // Extract realm URLs
    if let Some(realm_configs) = json_value["network"]["realm_configs"].as_array() {
        for config in realm_configs {
            if let Some(rpc_urls) = config["rpc_url"].as_array() {
                for url in rpc_urls {
                    if let Some(url_str) = url.as_str() {
                        edge_urls.push(url_str.to_string());
                    }
                }
            }
        }
    }

    if edge_urls.is_empty() {
        anyhow::bail!("No edge URLs (coordinator or realm) found in config file");
    }

    Ok(edge_urls)
}
