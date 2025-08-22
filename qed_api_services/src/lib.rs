pub mod config;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod services;
pub mod telemetry;
pub mod websocket;

pub use config::Config;

pub type Result<T> = anyhow::Result<T>;