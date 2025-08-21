use chrono::{DateTime, Utc};
use qed_core::job::id::QProvingJobDataID;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEvent {
    pub id: Option<Uuid>,
    pub realm_id: Option<i64>,
    pub public_key: Option<String>,
    pub status: String,
    pub source: String,
    pub job_id: QProvingJobDataID,
    pub checkpoint_id: i64,
    pub duration: Option<i64>, // milliseconds
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEvent {
    pub user_id: String,
    pub public_key: String,
    pub tx_type: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Option<Uuid>,
    pub public_key: String,
    pub twitter_handle: Option<String>,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Enumeration models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEventStatus {
    pub id: i32,
    pub status: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEventSource {
    pub id: i32,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEventTxType {
    pub id: i32,
    pub tx_type: String,
    pub created_at: DateTime<Utc>,
}

// Aggregation models for time-series data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEventAggregation {
    pub bucket: DateTime<Utc>,
    pub realm_id: Option<i64>,
    pub status: Option<String>,
    pub source: String,
    pub event_count: i64,
    pub avg_duration_ms: Option<f64>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub completed_count: i64,
    pub failed_count: i64,
    pub processing_count: i64,
    pub pending_count: i64,
    pub duration_recorded_count: i64,
    pub unique_checkpoints: i64,
    pub coordinator_events: i64,
    pub realm_events: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEventAggregation {
    pub bucket: DateTime<Utc>,
    pub tx_type: String,
    pub event_count: i64,
    pub unique_users: i64,
    pub unique_public_keys: i64,
}
