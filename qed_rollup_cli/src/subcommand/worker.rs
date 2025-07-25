use qed_node::worker::run_worker;
use tracing::info;

pub async fn run(edge_urls: Vec<String>) -> anyhow::Result<()> {
    info!("Worker starting...");
    info!("Worker args: {:?}", edge_urls);
    run_worker(edge_urls).await?;
    let _ = tokio::signal::ctrl_c().await;
    info!("Worker exit.");
    Ok(())
}
