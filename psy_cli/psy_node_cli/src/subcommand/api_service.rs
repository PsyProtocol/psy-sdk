use psy_services::{config::JwtConfig, run, Config};

pub async fn run_services(host: String, port: u16, database_url: String, max_connections: u32) -> anyhow::Result<()> {
    let config = Config {
        server: psy_services::config::ServerConfig { host, port },
        database: psy_services::config::DatabaseConfig {
            url: database_url,
            max_connections,
        },
        jwt: JwtConfig {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                tracing::warn!("JWT_SECRET not set in environment, using default (INSECURE!)");
                "your-secret-key-change-this-in-production".to_string()
            }),
            expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "3".to_string())
                .parse::<u32>()
                .unwrap_or(3),
        },
    };
    run(config).await
}
