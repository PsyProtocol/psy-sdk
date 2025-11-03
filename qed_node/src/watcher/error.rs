// error.rs - Centralized error handling
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WatcherError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("API communication error: {0}")]
    ApiClient(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Job timeout: {job_id}")]
    JobTimeout { job_id: String },

    #[error("Maximum retries exceeded: {0}")]
    MaxRetriesExceeded(String),

    #[error("Lock acquisition failed: {0}")]
    LockFailed(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}

pub type Result<T> = std::result::Result<T, WatcherError>;

impl From<anyhow::Error> for WatcherError {
    fn from(err: anyhow::Error) -> Self {
        WatcherError::Database(err.to_string())
    }
}