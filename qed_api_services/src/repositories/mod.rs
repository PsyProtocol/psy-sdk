use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::models::*;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create_user(&self, user: &UserInfo) -> crate::Result<UserInfo>;
    async fn get_user_by_public_key(&self, public_key: &str) -> crate::Result<Option<UserInfo>>;
}

#[async_trait]
pub trait WorkerEventRepository: Send + Sync {
    async fn insert_worker_event(&self, event: &WorkerEvent) -> crate::Result<()>;
    async fn get_worker_events(
        &self,
        realm_id: Option<&str>,
        status: Option<&str>,
        public_key: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> crate::Result<Vec<WorkerEvent>>;
    async fn get_worker_events_aggregation(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        bucket_size: &str,
    ) -> crate::Result<Vec<WorkerEventAggregation>>;
}

#[async_trait]
pub trait UserEventRepository: Send + Sync {
    async fn insert_user_event(&self, event: &UserEvent) -> crate::Result<()>;
    async fn get_user_events(
        &self,
        user_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        tx_type: Option<&str>,
    ) -> crate::Result<Vec<UserEvent>>;
    async fn get_user_events_aggregation(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        bucket_size: &str,
    ) -> crate::Result<Vec<UserEventAggregation>>;
}

// TODO: Implement PostgreSQL repository implementations