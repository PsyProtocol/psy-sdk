use thiserror::Error;
use qed_core::data::qhashout::QHashOut;
use psy_data::config::store_config::QEDFelt;

#[derive(Debug, Error)]
pub enum CoordinatorError {
    #[error("User already registered with id: {user_id}")]
    UserAlreadyRegistered { user_id: u64 },
    
    #[error("User not found for public key: {public_key:?}")]
    UserNotFound { public_key: QHashOut<QEDFelt> },
    
    #[error("Store error: {0}")]
    StoreError(#[from] anyhow::Error),
    
    #[error("Queue error: {0}")]
    QueueError(String),
    
    #[error("Invalid checkpoint: requested {requested}, latest is {latest}")]
    InvalidCheckpoint { requested: u64, latest: u64 },
}