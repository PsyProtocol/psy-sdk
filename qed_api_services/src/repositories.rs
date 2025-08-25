use chrono::{DateTime, Utc};
use qed_core::job::id::QProvingJobDataID;
use sqlx::PgPool;

use crate::models::{
    job_id_to_json, GlobalRealmStats, RealmStats, UserEvent, UserEventAggregation, UserEventTxType,
    UserInfo, WorkerEvent, WorkerEventAggregation, WorkerEventSource, WorkerEventStatus,
    WorkerStats,
};
use crate::Result;

pub struct UserRepository;
pub struct WorkerEventRepository;
pub struct UserEventRepository;
pub struct WorkerEventAggregationRepository;
pub struct UserEventAggregationRepository;
pub struct RealmStatsRepository;
pub struct WorkerStatsRepository;

impl UserRepository {
    /// Create a new user
    pub async fn create(
        pool: &PgPool,
        public_key: &str,
        twitter_handle: Option<&str>,
        label: Option<&str>,
    ) -> Result<UserInfo> {
        let row = sqlx::query!(
            r#"
            INSERT INTO user_info (public_key, twitter_handle, label)
            VALUES ($1, $2, $3)
            RETURNING id, public_key, twitter_handle, label, created_at, updated_at
            "#,
            public_key,
            twitter_handle,
            label
        )
        .fetch_one(pool)
        .await?;

        Ok(UserInfo {
            id: Some(row.id),
            public_key: row.public_key,
            twitter_handle: row.twitter_handle,
            label: row.label,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Find user by public key
    pub async fn find_by_public_key(pool: &PgPool, public_key: &str) -> Result<Option<UserInfo>> {
        let row = sqlx::query!(
            r#"
            SELECT id, public_key, twitter_handle, label, created_at, updated_at
            FROM user_info
            WHERE public_key = $1
            "#,
            public_key
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| UserInfo {
            id: Some(r.id),
            public_key: r.public_key,
            twitter_handle: r.twitter_handle,
            label: r.label,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// Get all users with pagination
    pub async fn list(pool: &PgPool, offset: i64, limit: i64) -> Result<Vec<UserInfo>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, public_key, twitter_handle, label, created_at, updated_at
            FROM user_info
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let users = rows
            .into_iter()
            .map(|r| UserInfo {
                id: Some(r.id),
                public_key: r.public_key,
                twitter_handle: r.twitter_handle,
                label: r.label,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect();

        Ok(users)
    }

    /// Update user info
    pub async fn update(pool: &PgPool, user_info: &UserInfo) -> Result<()> {
        let _ = sqlx::query!(
            r#"
            UPDATE user_info
            SET twitter_handle = $2, label = $3, updated_at = NOW()
            WHERE public_key = $1
            RETURNING id, public_key, twitter_handle, label, created_at, updated_at
            "#,
            user_info.public_key,
            user_info.twitter_handle,
            user_info.label
        )
        .fetch_one(pool)
        .await?;
        Ok(())
    }
}

impl WorkerEventRepository {
    /// Create a new worker event
    pub async fn create(
        pool: &PgPool,
        realm_id: Option<i64>,
        public_key: Option<&str>,
        status: WorkerEventStatus,
        source: WorkerEventSource,
        job_id: &QProvingJobDataID,
        checkpoint_id: i64,
        duration: Option<i64>,
        metadata: Option<&serde_json::Value>,
        timestamp: DateTime<Utc>,
    ) -> Result<WorkerEvent> {
        let job_id_json = job_id_to_json(job_id);
        let default_metadata = serde_json::json!({});
        let metadata_value = metadata.unwrap_or(&default_metadata);

        let row = sqlx::query!(
            r#"
            INSERT INTO worker_events
            (realm_id, public_key, status, source, job_id, checkpoint_id, duration, metadata, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, realm_id, public_key, status, source,
                job_id, checkpoint_id, duration, metadata, timestamp, created_at, updated_at
            "#,
            realm_id,
            public_key,
            status as WorkerEventStatus,
            source as WorkerEventSource,
            &job_id_json,
            checkpoint_id,
            duration,
            metadata_value,
            timestamp
        )
        .fetch_one(pool)
        .await?;

        let parsed_job_id = crate::models::job_id_from_json(row.job_id)?;

        Ok(WorkerEvent {
            id: Some(row.id),
            realm_id: row.realm_id,
            public_key: row.public_key,
            status: row.status.parse().unwrap(),
            source: row.source.parse().unwrap(),
            job_id: parsed_job_id,
            checkpoint_id: row.checkpoint_id,
            duration: row.duration,
            metadata: row.metadata,
            timestamp: row.timestamp,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Get worker events with filtering and pagination
    /// Note: Uses dynamic query due to complex optional filtering
    pub async fn list(
        pool: &PgPool,
        realm_id: Option<i64>,
        status: Option<WorkerEventStatus>,
        source: Option<WorkerEventSource>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<WorkerEvent>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id, realm_id, public_key, status, source,
                job_id, checkpoint_id, duration, metadata, timestamp, created_at, updated_at
            FROM worker_events
            WHERE ($1::BIGINT IS NULL OR realm_id = $1)
                AND ($2::VARCHAR IS NULL OR status = $2)
                AND ($3::VARCHAR IS NULL OR source = $3)
                AND ($4::TIMESTAMPTZ IS NULL OR timestamp >= $4)
                AND ($5::TIMESTAMPTZ IS NULL OR timestamp <= $5)
            ORDER BY timestamp DESC
            LIMIT $6 OFFSET $7
            "#,
            realm_id,
            status.map(|s| s.to_string()),
            source.map(|s| s.to_string()),
            start_time,
            end_time,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        use crate::models::job_id_from_json;
        use qed_core::job::id::ProvingJobCircuitType;

        // Create a default QProvingJobDataID in case of conversion failure
        let default_job_id =
            QProvingJobDataID::new_proof_job_id(0, ProvingJobCircuitType::AddL1Deposit, 0, 0, 0);

        let events = rows
            .into_iter()
            .map(|row| {
                let parsed_job_id = job_id_from_json(row.job_id).unwrap_or(default_job_id.clone());

                WorkerEvent {
                    id: Some(row.id),
                    realm_id: row.realm_id,
                    public_key: row.public_key,
                    status: row.status.parse().unwrap(),
                    source: row.source.parse().unwrap(),
                    job_id: parsed_job_id,
                    checkpoint_id: row.checkpoint_id,
                    duration: row.duration,
                    metadata: row.metadata,
                    timestamp: row.timestamp,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }
            })
            .collect();

        Ok(events)
    }

    /// Get worker events count with filtering
    pub async fn count(
        pool: &PgPool,
        realm_id: Option<i64>,
        status: Option<WorkerEventStatus>,
        source: Option<WorkerEventSource>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM worker_events
            WHERE ($1::BIGINT IS NULL OR realm_id = $1)
                AND ($2::VARCHAR IS NULL OR status = $2)
                AND ($3::VARCHAR IS NULL OR source = $3)
                AND ($4::TIMESTAMPTZ IS NULL OR timestamp >= $4)
                AND ($5::TIMESTAMPTZ IS NULL OR timestamp <= $5)
            "#,
            realm_id,
            status.map(|s| s.to_string()),
            source.map(|s| s.to_string()),
            start_time,
            end_time
        )
        .fetch_one(pool)
        .await?;

        Ok(row.count.unwrap_or(0))
    }
}

/// User Event Queries
impl UserEventRepository {
    /// Create a new user event
    pub async fn create(
        pool: &PgPool,
        user_id: &str,
        public_key: &str,
        tx_type: UserEventTxType,
        metadata: Option<&serde_json::Value>,
        timestamp: DateTime<Utc>,
    ) -> Result<UserEvent> {
        let default_metadata = serde_json::json!({});
        let metadata_value = metadata.unwrap_or(&default_metadata);

        let row = sqlx::query!(
            r#"
            INSERT INTO user_events (user_id, public_key, tx_type, metadata, timestamp)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                user_id, public_key, tx_type,
                metadata, timestamp, created_at, updated_at
            "#,
            user_id,
            public_key,
            tx_type as UserEventTxType,
            metadata_value,
            timestamp
        )
        .fetch_one(pool)
        .await?;

        Ok(UserEvent {
            user_id: row.user_id,
            public_key: row.public_key,
            tx_type: row.tx_type.parse().unwrap(),
            metadata: row.metadata,
            timestamp: row.timestamp,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Get user events with filtering and pagination
    pub async fn list(
        pool: &PgPool,
        user_id: Option<&str>,
        public_key: Option<&str>,
        tx_type: Option<UserEventTxType>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<UserEvent>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                user_id, public_key, tx_type,
                metadata, timestamp, created_at, updated_at
            FROM user_events
            WHERE ($1::VARCHAR IS NULL OR user_id = $1)
                AND ($2::VARCHAR IS NULL OR public_key = $2)
                AND ($3::VARCHAR IS NULL OR tx_type = $3)
                AND ($4::TIMESTAMPTZ IS NULL OR timestamp >= $4)
                AND ($5::TIMESTAMPTZ IS NULL OR timestamp <= $5)
            ORDER BY timestamp DESC
            LIMIT $6 OFFSET $7
            "#,
            user_id,
            public_key,
            tx_type.map(|t| t.to_string()),
            start_time,
            end_time,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let events = rows
            .into_iter()
            .map(|row| UserEvent {
                user_id: row.user_id,
                public_key: row.public_key,
                tx_type: row.tx_type.parse().unwrap(),
                metadata: row.metadata,
                timestamp: row.timestamp,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(events)
    }
}

/// Worker Event Aggregation Queries
impl WorkerEventAggregationRepository {
    /// Get worker event aggregations from materialized views
    /// Note: Uses dynamic query for flexible view selection
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        realm_id: Option<i64>,
        source: Option<WorkerEventSource>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<WorkerEventAggregation>> {
        // Note: view_name should be validated against a whitelist in production
        let query = format!(
            r#"
            SELECT
                bucket, realm_id, source,
                count, completed_count, failed_count, processing_count, pending_count,
                avg_duration_ms, min_duration_ms, max_duration_ms
            FROM {}
            WHERE ($1::BIGINT IS NULL OR realm_id = $1)
                AND ($2::VARCHAR IS NULL OR source = $2)
                AND ($3::TIMESTAMPTZ IS NULL OR bucket >= $3)
                AND ($4::TIMESTAMPTZ IS NULL OR bucket <= $4)
            ORDER BY bucket DESC
            LIMIT $5
            "#,
            view_name
        );

        let aggregations = sqlx::query_as::<_, WorkerEventAggregation>(&query)
            .bind(realm_id)
            .bind(source)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .fetch_all(pool)
            .await?;

        Ok(aggregations)
    }
}

/// User Event Aggregation Queries
impl UserEventAggregationRepository {
    /// Get user event aggregations from materialized views
    /// Note: Uses dynamic query for flexible view selection
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<UserEventAggregation>> {
        let query = format!(
            r#"
            SELECT
                bucket, count, register_user_count, deploy_contract_count, guta_count
            FROM {}
            WHERE ($1::TIMESTAMPTZ IS NULL OR bucket >= $1)
                AND ($2::TIMESTAMPTZ IS NULL OR bucket <= $2)
            ORDER BY bucket DESC
            LIMIT $3
            "#,
            view_name
        );

        let aggregations = sqlx::query_as::<_, UserEventAggregation>(&query)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .fetch_all(pool)
            .await?;

        Ok(aggregations)
    }
}

/// Realm Statistics Repository
impl RealmStatsRepository {
    /// Get statistics for a specific realm
    pub async fn get_realm_stats(pool: &PgPool, realm_id: i64) -> Result<RealmStats> {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);

        // Get processing tasks count directly from worker_events (real-time)
        let processing_tasks_row = sqlx::query!(
            r#"
            SELECT COUNT(*) as processing_tasks
            FROM worker_events
            WHERE realm_id = $1 AND status = 'PROCESSING'
            "#,
            realm_id
        )
        .fetch_one(pool)
        .await?;

        let processing_tasks = processing_tasks_row.processing_tasks.unwrap_or(0);

        // Get active workers and users for 1h (direct query)
        let active_1h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_1h,
                COUNT(DISTINCT CASE WHEN source = 'REALM' THEN public_key END) as active_users_1h
            FROM worker_events
            WHERE realm_id = $1 AND timestamp >= $2
            "#,
            realm_id,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_1h = active_1h_row.active_workers_1h.unwrap_or(0);
        let active_users_1h = active_1h_row.active_users_1h.unwrap_or(0);

        // Get active workers and users for 24h (direct query)
        let active_24h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_24h,
                COUNT(DISTINCT CASE WHEN source = 'REALM' THEN public_key END) as active_users_24h
            FROM worker_events
            WHERE realm_id = $1 AND timestamp >= $2
            "#,
            realm_id,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_24h = active_24h_row.active_workers_24h.unwrap_or(0);
        let active_users_24h = active_24h_row.active_users_24h.unwrap_or(0);

        Ok(RealmStats {
            realm_id,
            processing_tasks,
            active_workers_1h,
            active_workers_24h,
            active_users_1h,
            active_users_24h,
            last_updated: now,
        })
    }

    /// Get global statistics across all realms
    pub async fn get_global_realm_stats(pool: &PgPool) -> Result<GlobalRealmStats> {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);

        // Get total processing tasks across all realms (real-time)
        let total_processing_row = sqlx::query!(
            r#"
            SELECT COUNT(*) as total_processing_tasks
            FROM worker_events
            WHERE status = 'PROCESSING'
            "#
        )
        .fetch_one(pool)
        .await?;

        let total_processing_tasks = total_processing_row.total_processing_tasks.unwrap_or(0);

        // Get 1h stats (direct query)
        let active_1h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT realm_id) FILTER (WHERE realm_id IS NOT NULL) as active_realms_1h,
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_1h,
                COUNT(DISTINCT CASE WHEN source = 'REALM' THEN public_key END) as active_users_1h
            FROM worker_events
            WHERE timestamp >= $1
            "#,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_1h = active_1h_row.active_workers_1h.unwrap_or(0);
        let active_users_1h = active_1h_row.active_users_1h.unwrap_or(0);
        let active_realms_1h = active_1h_row.active_realms_1h.unwrap_or(0);

        // Get 24h stats (direct query)
        let active_24h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT realm_id) FILTER (WHERE realm_id IS NOT NULL) as active_realms_24h,
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_24h,
                COUNT(DISTINCT CASE WHEN source = 'REALM' THEN public_key END) as active_users_24h
            FROM worker_events
            WHERE timestamp >= $1
            "#,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_24h = active_24h_row.active_workers_24h.unwrap_or(0);
        let active_users_24h = active_24h_row.active_users_24h.unwrap_or(0);
        let active_realms_24h = active_24h_row.active_realms_24h.unwrap_or(0);

        Ok(GlobalRealmStats {
            total_processing_tasks,
            active_workers_1h,
            active_workers_24h,
            active_users_1h,
            active_users_24h,
            active_realms_1h,
            active_realms_24h,
            last_updated: now,
        })
    }
}

/// Worker Statistics Repository
impl WorkerStatsRepository {
    /// Get statistics for a specific worker by public key
    pub async fn get_worker_stats(pool: &PgPool, worker_public_key: &str) -> Result<WorkerStats> {
        let now = Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);

        // Get processing tasks count grouped by realm_id
        let processing_tasks_rows = sqlx::query!(
            r#"
            SELECT realm_id, COUNT(*) as task_count
            FROM worker_events
            WHERE public_key = $1 AND status = 'PROCESSING'
            GROUP BY realm_id
            "#,
            worker_public_key
        )
        .fetch_all(pool)
        .await?;

        let mut processing_tasks = std::collections::HashMap::new();
        let mut total_processing_tasks = 0i64;

        for row in processing_tasks_rows {
            let realm_key = match row.realm_id {
                Some(id) => format!("realm{}", id),
                None => "realm_unknown".to_string(),
            };
            let count = row.task_count.unwrap_or(0);
            processing_tasks.insert(realm_key, count);
            total_processing_tasks += count;
        }

        // Get completed and failed tasks in the last 24h
        let completion_stats_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as completed_24h,
                COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as failed_24h,
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as total_proofs
            FROM worker_events
            WHERE public_key = $1 AND timestamp >= $2
            "#,
            worker_public_key,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let completed_24h = completion_stats_row.completed_24h.unwrap_or(0);
        let failed_24h = completion_stats_row.failed_24h.unwrap_or(0);
        let total_proofs = completion_stats_row.total_proofs.unwrap_or(0);

        // Currently, the total rewards is 0
        let total_rewards = 0i64;

        Ok(WorkerStats {
            processing_tasks,
            total_processing_tasks,
            total_rewards,
            total_proofs,
            completed_24h,
            failed_24h,
            last_updated: now,
        })
    }
}
