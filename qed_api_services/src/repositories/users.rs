use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::models::{UserEvent, UserEventTxType, UserInfo};
use crate::Result;

pub struct UserRepository;
pub struct UserEventRepository;

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

        let tx_type = row.tx_type
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse tx_type: {}", e))?;

        Ok(UserEvent {
            user_id: row.user_id,
            public_key: row.public_key,
            tx_type,
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
        order_asc: bool,
    ) -> Result<Vec<UserEvent>> {
        let order_direction = if order_asc { "ASC" } else { "DESC" };

        let query_str = format!(
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
            ORDER BY timestamp {}
            LIMIT $6 OFFSET $7
            "#,
            order_direction
        );

        let rows = sqlx::query(&query_str)
            .bind(user_id)
            .bind(public_key)
            .bind(tx_type.map(|t| t.to_string()))
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

        use sqlx::Row;

        let events: Result<Vec<UserEvent>> = rows
            .into_iter()
            .map(|row| {
                let tx_type = row.get::<String, _>("tx_type")
                    .parse()
                    .map_err(|e| anyhow::anyhow!("Failed to parse tx_type: {}", e))?;

                Ok(UserEvent {
                    user_id: row.get("user_id"),
                    public_key: row.get("public_key"),
                    tx_type,
                    metadata: row.get("metadata"),
                    timestamp: row.get("timestamp"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            })
            .collect();

        events
    }

    /// Get user events count with filtering
    pub async fn count(
        pool: &PgPool,
        user_id: Option<&str>,
        public_key: Option<&str>,
        tx_type: Option<UserEventTxType>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM user_events
            WHERE ($1::VARCHAR IS NULL OR user_id = $1)
                AND ($2::VARCHAR IS NULL OR public_key = $2)
                AND ($3::VARCHAR IS NULL OR tx_type = $3)
                AND ($4::TIMESTAMPTZ IS NULL OR timestamp >= $4)
                AND ($5::TIMESTAMPTZ IS NULL OR timestamp <= $5)
            "#,
            user_id,
            public_key,
            tx_type.map(|t| t.to_string()),
            start_time,
            end_time
        )
        .fetch_one(pool)
        .await?;

        Ok(row.count.unwrap_or(0))
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

        let events: Result<Vec<UserEvent>> = rows
            .into_iter()
            .map(|row| {
                let tx_type = row.tx_type
                    .parse()
                    .map_err(|e| anyhow::anyhow!("Failed to parse tx_type: {}", e))?;

                Ok(UserEvent {
                    user_id: row.user_id,
                    public_key: row.public_key,
                    tx_type,
                    metadata: row.metadata,
                    timestamp: row.timestamp,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
            })
            .collect();

        events
    }
}