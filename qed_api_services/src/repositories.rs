use chrono::{DateTime, Utc};
use qed_core::job::id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID};
use sqlx::PgPool;

use crate::models::{
    job_id_to_json, GlobalRealmStats, RealmStats, TpsData, UserEvent, UserEventAggregation,
    UserEventTxType, UserInfo, WorkerEvent, WorkerEventAggregation, WorkerEventReward,
    WorkerEventSource, WorkerEventStatus, WorkerRewards, WorkerStats,
};
use crate::Result;

pub struct UserRepository;
pub struct WorkerEventRepository;
pub struct UserEventRepository;
pub struct WorkerEventAggregationRepository;
pub struct UserEventAggregationRepository;
pub struct RealmStatsRepository;
pub struct WorkerStatsRepository;
pub struct WorkerRewardsRepository;
pub struct WorkerEventRewardRepository;
pub struct TpsRepository;

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
        let topic = job_id.topic.to_u8() as i16;
        let circuit_type = job_id.circuit_type.to_u8() as i16;

        let row = sqlx::query!(
            r#"
            INSERT INTO worker_events
            (realm_id, public_key, status, source, job_id, topic, circuit_type, checkpoint_id, duration, metadata, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id, realm_id, public_key, status, source,
                job_id, topic, circuit_type, checkpoint_id, duration, metadata, timestamp, created_at, updated_at
            "#,
            realm_id,
            public_key,
            status as WorkerEventStatus,
            source as WorkerEventSource,
            &job_id_json,
            topic,
            circuit_type,
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
        topic: Option<QJobTopic>,
        circuit_type: Option<ProvingJobCircuitType>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<WorkerEvent>> {
        let topic = topic.map(|t| t.to_u8() as i16);
        let circuit_type = circuit_type.map(|t| t.to_u8() as i16);
        let rows = sqlx::query!(
            r#"
            SELECT
                id, realm_id, public_key, status, source,
                job_id, checkpoint_id, duration, metadata, timestamp, created_at, updated_at
            FROM worker_events
            WHERE ($1::BIGINT IS NULL OR realm_id = $1)
                AND ($2::VARCHAR IS NULL OR status = $2)
                AND ($3::VARCHAR IS NULL OR source = $3)
                AND ($4::SMALLINT IS NULL OR topic = $4)
                AND ($5::SMALLINT IS NULL OR circuit_type = $5)
                AND ($6::TIMESTAMPTZ IS NULL OR timestamp >= $6)
                AND ($7::TIMESTAMPTZ IS NULL OR timestamp <= $7)
            ORDER BY timestamp DESC
            LIMIT $8 OFFSET $9
            "#,
            realm_id,
            status.map(|s| s.to_string()),
            source.map(|s| s.to_string()),
            topic,
            circuit_type,
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
            QProvingJobDataID::new_proof_job_id(0, 0, ProvingJobCircuitType::AddL1Deposit, 0, 0);

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
        topic: Option<QJobTopic>,
        circuit_type: Option<ProvingJobCircuitType>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let topic = topic.map(|t| t.to_u8() as i16);
        let circuit_type = circuit_type.map(|t| t.to_u8() as i16);
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM worker_events
            WHERE ($1::BIGINT IS NULL OR realm_id = $1)
                AND ($2::VARCHAR IS NULL OR status = $2)
                AND ($3::VARCHAR IS NULL OR source = $3)
                AND ($4::SMALLINT IS NULL OR topic = $4)
                AND ($5::SMALLINT IS NULL OR circuit_type = $5)
                AND ($6::TIMESTAMPTZ IS NULL OR timestamp >= $6)
                AND ($7::TIMESTAMPTZ IS NULL OR timestamp <= $7)
            "#,
            realm_id,
            status.map(|s| s.to_string()),
            source.map(|s| s.to_string()),
            topic,
            circuit_type,
            start_time,
            end_time
        )
        .fetch_one(pool)
        .await?;

        Ok(row.count.unwrap_or(0))
    }

    /// Get GUTA-related worker events that don't have rewards yet
    pub async fn get_unprocessed_guta_worker_events(
        pool: &PgPool,
        checkpoint_range: Option<(i64, i64)>, // (min_checkpoint, max_checkpoint)
    ) -> Result<Vec<WorkerEvent>> {
        // Circuit types for GUTA-related events based on your requirements
        let guta_circuit_types = vec![
            ProvingJobCircuitType::GUTAOnlyRegisterUsers.to_u8() as i16,
            ProvingJobCircuitType::GUTARegisterUsers.to_u8() as i16,
            ProvingJobCircuitType::GUTATwoEndCap.to_u8() as i16,
            ProvingJobCircuitType::GUTATwoGUTA.to_u8() as i16,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA.to_u8() as i16,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap.to_u8() as i16,
            ProvingJobCircuitType::GUTASingleEndCap.to_u8() as i16,
            ProvingJobCircuitType::GUTAVerifyToCap.to_u8() as i16,
            ProvingJobCircuitType::GUTANoChange.to_u8() as i16,
        ];

        let (min_checkpoint, max_checkpoint) = checkpoint_range.unwrap_or((0, i64::MAX));

        let rows = sqlx::query!(
            r#"
            SELECT
                we.id, we.realm_id, we.public_key, we.status, we.source,
                we.job_id, we.topic, we.circuit_type, we.checkpoint_id,
                we.duration, we.metadata, we.timestamp, we.created_at, we.updated_at
            FROM worker_events we
            LEFT JOIN worker_event_rewards wer ON we.id = wer.id
            WHERE we.circuit_type = ANY($1::SMALLINT[])
                AND we.status = 'COMPLETED'
                AND we.checkpoint_id >= $2
                AND we.checkpoint_id <= $3
                AND wer.id IS NULL
            ORDER BY we.checkpoint_id ASC, we.timestamp ASC
            "#,
            &guta_circuit_types,
            min_checkpoint,
            max_checkpoint
        )
        .fetch_all(pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let parsed_job_id = crate::models::job_id_from_json(row.job_id)?;
            events.push(WorkerEvent {
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
            });
        }

        Ok(events)
    }

    /// Get the maximum checkpoint_id from worker_events (for reward processing)
    pub async fn get_max_checkpoint(pool: &PgPool) -> Result<Option<i64>> {
        let result = sqlx::query!(
            r#"
            SELECT MAX(checkpoint_id) as max_checkpoint
            FROM worker_events
            "#
        )
        .fetch_one(pool)
        .await?;

        Ok(result.max_checkpoint)
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

    /// Get GUTA user events for a specific checkpoint (for reward calculation)
    pub async fn get_guta_events_by_checkpoint(
        pool: &PgPool,
        checkpoint_id: i64,
    ) -> Result<Vec<UserEvent>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                user_id, public_key, tx_type,
                metadata, timestamp, created_at, updated_at
            FROM user_events
            WHERE tx_type = 'GUTA'
                AND metadata->>'checkpoint_id' = $1::text
            ORDER BY timestamp DESC
            "#,
            checkpoint_id.to_string()
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
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_1h
            FROM worker_events
            WHERE timestamp >= $1
            "#,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;
        let active_workers_1h = active_1h_row.active_workers_1h.unwrap_or(0);
        let active_realms_1h = active_1h_row.active_realms_1h.unwrap_or(0);

        let active_1h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_users_1h
            FROM user_events
            WHERE timestamp >= $1
            "#,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;
        let active_users_1h = active_1h_row.active_users_1h.unwrap_or(0);

        // Get 24h stats (direct query)
        let active_24h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT realm_id) FILTER (WHERE realm_id IS NOT NULL) as active_realms_24h,
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_24h
            FROM worker_events
            WHERE timestamp >= $1
            "#,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_24h = active_24h_row.active_workers_24h.unwrap_or(0);
        let active_realms_24h = active_24h_row.active_realms_24h.unwrap_or(0);

        let active_24h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_users_24h
            FROM user_events
            WHERE timestamp >= $1
            "#,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;
        let active_users_24h = active_24h_row.active_users_24h.unwrap();

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
        let one_hour_ago = now - chrono::Duration::hours(1);

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

        // Get completed and failed tasks in the last 1h
        let completion_1h_stats_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as completed_1h,
                COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as failed_1h
            FROM worker_events
            WHERE public_key = $1 AND timestamp >= $2
            "#,
            worker_public_key,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;

        let completed_1h = completion_1h_stats_row.completed_1h.unwrap_or(0);
        let failed_1h = completion_1h_stats_row.failed_1h.unwrap_or(0);

        // Get total completed and failed tasks of all time
        let total_completion_stats_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as total_completed,
                COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as total_failed
            FROM worker_events
            WHERE public_key = $1
            "#,
            worker_public_key
        )
        .fetch_one(pool)
        .await?;

        let total_completed = total_completion_stats_row.total_completed.unwrap_or(0);
        let total_failed = total_completion_stats_row.total_failed.unwrap_or(0);

        // Calculate total rewards in the last 24 hours
        // Only count rewards for GenerateStandardProof jobs (topic = 0) with COMPLETED status
        const REWARD_PER_PROOF: i64 = 5_000_000_000; // 5*10^9 psy
        const TOPIC_GENERATE_STANDARD_PROOF: i16 = 0;

        let rewards_24h_row = sqlx::query!(
            r#"
            SELECT COUNT(*) as reward_proofs_24h
            FROM worker_events
            WHERE public_key = $1
                AND topic = $2
                AND status = 'COMPLETED'
                AND timestamp >= $3
            "#,
            worker_public_key,
            TOPIC_GENERATE_STANDARD_PROOF,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let reward_proofs_24h = rewards_24h_row.reward_proofs_24h.unwrap_or(0);
        let total_rewards_24h = reward_proofs_24h * REWARD_PER_PROOF;

        // Currently, the total rewards is 0 (reserved field)
        let total_rewards = 0i64;

        Ok(WorkerStats {
            processing_tasks,
            total_processing_tasks,
            total_rewards,
            total_proofs,
            completed_24h,
            failed_24h,
            total_rewards_24h,
            total_completed,
            total_failed,
            completed_1h,
            failed_1h,
            last_updated: now,
        })
    }
}

/// Worker Rewards Repository
impl WorkerRewardsRepository {
    /// Get rewards for a specific worker by public key and checkpoint_id
    pub async fn get_worker_rewards(
        pool: &PgPool,
        worker_public_key: &str,
        checkpoint_id: i64,
    ) -> Result<WorkerRewards> {
        let now = Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);
        let seven_days_ago = now - chrono::Duration::days(7);
        let thirty_days_ago = now - chrono::Duration::days(30);

        // Get rewards and proofs counts by checkpoint (claimed vs unclaimed)
        let rewards_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN checkpoint_id < $2 THEN 1 END)::BIGINT as claimed_proofs,
                COUNT(CASE WHEN checkpoint_id >= $2 THEN 1 END)::BIGINT as unclaimed_proofs,
                COUNT(*)::BIGINT as total_proofs,
                COALESCE(SUM(CASE WHEN checkpoint_id < $2 THEN reward_amount END), 0)::BIGINT as claimed_rewards,
                COALESCE(SUM(CASE WHEN checkpoint_id >= $2 THEN reward_amount END), 0)::BIGINT as unclaimed_rewards,
                COALESCE(SUM(reward_amount), 0)::BIGINT as total_rewards
            FROM worker_event_rewards
            WHERE public_key = $1
            "#,
            worker_public_key,
            checkpoint_id
        )
        .fetch_one(pool)
        .await?;

        let claimed_proofs = rewards_row.claimed_proofs.unwrap_or(0);
        let unclaimed_proofs = rewards_row.unclaimed_proofs.unwrap_or(0);
        let total_proofs = rewards_row.total_proofs.unwrap_or(0);
        let claimed_rewards = rewards_row.claimed_rewards.unwrap_or(0);
        let unclaimed_rewards = rewards_row.unclaimed_rewards.unwrap_or(0);
        let total_rewards = rewards_row.total_rewards.unwrap_or(0);

        // Query for time-based rewards (24h, 7d, 30d) from worker_event_rewards
        let time_rewards_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN timestamp >= $2 THEN 1 END)::BIGINT as proofs_24h,
                COUNT(CASE WHEN timestamp >= $3 THEN 1 END)::BIGINT as proofs_7d,
                COUNT(CASE WHEN timestamp >= $4 THEN 1 END)::BIGINT as proofs_30d,
                COALESCE(SUM(CASE WHEN timestamp >= $2 THEN reward_amount END), 0)::BIGINT as total_rewards_24h,
                COALESCE(SUM(CASE WHEN timestamp >= $3 THEN reward_amount END), 0)::BIGINT as total_rewards_7d,
                COALESCE(SUM(CASE WHEN timestamp >= $4 THEN reward_amount END), 0)::BIGINT as total_rewards_30d
            FROM worker_event_rewards
            WHERE public_key = $1
            "#,
            worker_public_key,
            twenty_four_hours_ago,
            seven_days_ago,
            thirty_days_ago
        )
        .fetch_one(pool)
        .await?;

        let total_rewards_24h = time_rewards_row.total_rewards_24h.unwrap_or(0);
        let total_rewards_7d = time_rewards_row.total_rewards_7d.unwrap_or(0);
        let total_rewards_30d = time_rewards_row.total_rewards_30d.unwrap_or(0);

        Ok(WorkerRewards {
            worker_public_key: worker_public_key.to_string(),
            checkpoint_id,
            claimed_rewards,
            unclaimed_rewards,
            total_rewards,
            claimed_proofs,
            unclaimed_proofs,
            total_proofs,
            total_rewards_24h,
            total_rewards_7d,
            total_rewards_30d,
            last_updated: now,
        })
    }
}

/// TPS Repository
impl TpsRepository {
    /// Get the maximum checkpoint ID from worker_events table
    pub async fn get_max_checkpoint(pool: &PgPool) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            SELECT MAX(checkpoint_id) as max_checkpoint
            FROM worker_events
            "#
        )
        .fetch_one(pool)
        .await?;

        Ok(row.max_checkpoint.unwrap_or(0))
    }

    /// Calculate current TPS based on the last 12 seconds of user events
    pub async fn calculate_current_tps(pool: &PgPool) -> Result<TpsData> {
        let now = Utc::now();
        let twelve_seconds_ago = now - chrono::Duration::seconds(12);
        const TIME_WINDOW_SECONDS: i64 = 12;

        // Query all user_events from the last 12 seconds with their metadata
        let events = sqlx::query_as!(
            UserEvent,
            r#"
            SELECT
                user_id, public_key, tx_type as "tx_type: UserEventTxType",
                metadata, timestamp, created_at, updated_at
            FROM user_events
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp DESC
            "#,
            twelve_seconds_ago,
            now
        )
        .fetch_all(pool)
        .await?;

        // Calculate total transaction count by examining each event individually
        let mut total_transaction_count = 0i64;

        for event in &events {
            total_transaction_count += event.get_transaction_count();
        }

        // Calculate TPS
        let tps = total_transaction_count as f64 / TIME_WINDOW_SECONDS as f64;

        // Get the current block height (max checkpoint)
        let block_height = Self::get_max_checkpoint(pool).await?;

        Ok(TpsData {
            tps,
            transaction_count: total_transaction_count,
            time_window_seconds: TIME_WINDOW_SECONDS,
            block_height,
            timestamp: now,
        })
    }
}

/// Worker Event Reward Repository
impl WorkerEventRewardRepository {
    /// Insert worker event rewards
    pub async fn create_rewards(pool: &PgPool, rewards: &[WorkerEventReward]) -> Result<()> {
        for reward in rewards {
            sqlx::query!(
                r#"
                INSERT INTO worker_event_rewards
                    (id, public_key, checkpoint_id, reward_amount, timestamp)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (id) DO NOTHING
                "#,
                reward.id,
                reward.public_key,
                reward.checkpoint_id,
                reward.reward_amount,
                reward.timestamp
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// Check if a worker event already has a reward
    pub async fn has_reward(pool: &PgPool, worker_event_id: uuid::Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM worker_event_rewards
                WHERE id = $1
            ) as exists
            "#,
            worker_event_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.exists.unwrap_or(false))
    }

    /// Get rewards for worker events by event IDs
    pub async fn get_rewards_by_event_ids(
        pool: &PgPool,
        worker_event_ids: &[uuid::Uuid],
    ) -> Result<Vec<WorkerEventReward>> {
        let rewards = sqlx::query_as!(
            WorkerEventReward,
            r#"
            SELECT id, public_key, checkpoint_id, reward_amount, timestamp, created_at, updated_at
            FROM worker_event_rewards
            WHERE id = ANY($1)
            ORDER BY timestamp DESC
            "#,
            worker_event_ids
        )
        .fetch_all(pool)
        .await?;

        Ok(rewards)
    }

    /// Get all rewards for worker events in a checkpoint range
    pub async fn get_rewards_by_checkpoint_range(
        pool: &PgPool,
        start_checkpoint: i64,
        end_checkpoint: i64,
    ) -> Result<Vec<WorkerEventReward>> {
        let rewards = sqlx::query_as!(
            WorkerEventReward,
            r#"
            SELECT id, public_key, checkpoint_id, reward_amount,
                   timestamp, created_at, updated_at
            FROM worker_event_rewards
            WHERE checkpoint_id >= $1 AND checkpoint_id <= $2
            ORDER BY checkpoint_id DESC, timestamp DESC
            "#,
            start_checkpoint,
            end_checkpoint
        )
        .fetch_all(pool)
        .await?;

        Ok(rewards)
    }

    /// Get rewards for a specific worker (by public_key) in a checkpoint range
    pub async fn get_worker_rewards(
        pool: &PgPool,
        public_key: &str,
        start_checkpoint: Option<i64>,
        end_checkpoint: Option<i64>,
    ) -> Result<Vec<WorkerEventReward>> {
        let start = start_checkpoint.unwrap_or(0);
        let end = end_checkpoint.unwrap_or(i64::MAX);

        let rewards = sqlx::query_as!(
            WorkerEventReward,
            r#"
            SELECT id, public_key, checkpoint_id, reward_amount,
                   timestamp, created_at, updated_at
            FROM worker_event_rewards
            WHERE public_key = $1
                AND checkpoint_id >= $2
                AND checkpoint_id <= $3
            ORDER BY checkpoint_id DESC, timestamp DESC
            "#,
            public_key,
            start,
            end
        )
        .fetch_all(pool)
        .await?;

        Ok(rewards)
    }
}
