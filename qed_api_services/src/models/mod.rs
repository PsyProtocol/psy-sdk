use chrono::{DateTime, Utc};
use qed_core::job::id::QProvingJobDataID;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEvent {
    pub id: Option<Uuid>,
    pub location: String,
    pub action: String,
    pub job_id: QProvingJobDataID,
    pub msg_id: String,
    pub checkpoint_id: u64,
    pub timestamp: DateTime<Utc>,
    pub duration: Option<i64>, // milliseconds
    pub realm_id: Option<u64>,
    pub status: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEvent {
    pub id: Option<Uuid>,
    pub user_id: String,
    pub public_key: String,
    pub tx_type: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Option<Uuid>,
    pub public_key: String,
    pub twitter_handle: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

// TODO: Add aggregation models for time-series data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEventAggregation {
    pub bucket: DateTime<Utc>,
    pub count: i64,
    pub avg_duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEventAggregation {
    pub bucket: DateTime<Utc>,
    pub tx_type: String,
    pub count: i64,
}
