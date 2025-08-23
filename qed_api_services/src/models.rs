use chrono::{DateTime, Utc};
use qed_core::job::id::QProvingJobDataID;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
    sqlx::Type,
)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerEventStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
    sqlx::Type,
)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerEventSource {
    Coordinator,
    Realm,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
    sqlx::Type,
)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum UserEventTxType {
    RegisterUser,
    DeployContract,
    Guta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEvent {
    pub id: Option<Uuid>,
    pub realm_id: Option<i64>,
    pub public_key: Option<String>,
    pub status: WorkerEventStatus,
    pub source: WorkerEventSource,
    pub job_id: QProvingJobDataID,
    pub checkpoint_id: i64,
    pub duration: Option<i64>, // milliseconds
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserEvent {
    pub user_id: String,
    pub public_key: String,
    pub tx_type: UserEventTxType,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserInfo {
    pub id: Option<Uuid>,
    pub public_key: String,
    pub twitter_handle: Option<String>,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerEventAggregation {
    pub bucket: DateTime<Utc>,
    pub realm_id: Option<i64>,
    pub source: WorkerEventSource,
    pub count: i64,
    pub completed_count: i64,
    pub failed_count: i64,
    pub processing_count: i64,
    pub pending_count: i64,
    pub avg_duration_ms: Option<f64>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserEventAggregation {
    pub bucket: DateTime<Utc>,
    pub count: i64,
    pub register_user_count: i64,
    pub deploy_contract_count: i64,
    pub guta_count: i64,
}

// JSON conversion helper functions for QProvingJobDataID
pub fn job_id_from_json(value: serde_json::Value) -> Result<QProvingJobDataID, serde_json::Error> {
    serde_json::from_value(value)
}

pub fn job_id_to_json(job_id: &QProvingJobDataID) -> serde_json::Value {
    serde_json::to_value(job_id).unwrap_or(serde_json::Value::Null)
}

// Realm statistics models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmStats {
    pub realm_id: i64,
    pub processing_tasks: i64,
    pub active_workers_1h: i64,
    pub active_workers_24h: i64,
    pub active_users_1h: i64,
    pub active_users_24h: i64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRealmStats {
    pub total_processing_tasks: i64,
    pub active_workers_1h: i64,
    pub active_workers_24h: i64,
    pub active_users_1h: i64,
    pub active_users_24h: i64,
    pub active_realms_1h: i64,
    pub active_realms_24h: i64,
    pub last_updated: DateTime<Utc>,
}
