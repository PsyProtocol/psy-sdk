use psy_services::{run, Config};

pub async fn run_api_service(host: String, port: u16, database_url: String, max_connections: u32) -> anyhow::Result<()> {
    let config = Config {
        server: psy_services::config::ServerConfig { host, port },
        database: psy_services::config::DatabaseConfig {
            url: database_url,
            max_connections,
        },
    };
    run(config).await
}
