use anyhow::Result;
use qed_realm_node::{
    config::{setup_logging, RealmNodeConfig},
    edge::start_realm_edge_node,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from environment variables
    let config = RealmNodeConfig::load()?;

    // Initialize logging
    setup_logging(&config.log)?;

    // Decide whether to start Edge node or Processor node based on configuration
    if config.is_edge {
        // Start Edge node
        start_realm_edge_node(config).await?;
    } else {
        // Start Processor node - to be implemented
        println!("Processor node not implemented yet");
    }

    Ok(())
}
