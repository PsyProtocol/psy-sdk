use std::str::FromStr;

pub use tracing::Level;
use tracing_subscriber::{prelude::*, EnvFilter};

use chrono::{Duration, Utc};

pub fn setup_logging(log_level: String) -> anyhow::Result<()> {
    let log_level = Level::from_str(&log_level).unwrap_or(Level::INFO);
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{}", log_level)));

    let fmt_layer = if log_level < Level::INFO {
        tracing_subscriber::fmt::layer()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_ansi(true)
    } else {
        tracing_subscriber::fmt::layer()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_target(false)
            .with_ansi(true)
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}
