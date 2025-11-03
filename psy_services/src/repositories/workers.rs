use chrono::{DateTime, Utc};
use psy_common::job::id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID};
use sqlx::PgPool;

use crate::{
    models::{job_id_to_json, JobFilterCategory, WorkerEvent, WorkerEventSource, WorkerEventStatus},
    Result,
};

pub struct WorkerEventRepository;

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

        let status = row.status.parse().map_err(|e| anyhow::anyhow!("Failed to parse status: {}", e))?;

        let source = row.source.parse().map_err(|e| anyhow::anyhow!("Failed to parse source: {}", e))?;

        Ok(WorkerEvent {
            id: Some(row.id),
            realm_id: row.realm_id,
            public_key: row.public_key,
            status,
            source,
            job_id: parsed_job_id,
            checkpoint_id: row.checkpoint_id,
            duration: row.duration,
            metadata: row.metadata,
            timestamp: row.timestamp,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Get worker events with filtering and pagination by checkpoint range
    /// Note: Uses dynamic query due to complex optional filtering
    pub async fn list(
        pool: &PgPool,
        realm_id: Option<i64>,
        public_key: Option<String>,
        status: Option<WorkerEventStatus>,
        source: Option<WorkerEventSource>,
        topic: Option<QJobTopic>,
        circuit_type: Option<ProvingJobCircuitType>,
        from_checkpoint_id: Option<i64>,
        to_checkpoint_id: Option<i64>,
        filter_category: JobFilterCategory,
        offset: i64,
        limit: i64,
        order_asc: bool,
    ) -> Result<Vec<WorkerEvent>> {
        let topic = topic.map(|t| t.to_u8() as i16);
        let circuit_type = circuit_type.map(|t| t.to_u8() as i16);
        let order_direction = if order_asc { "ASC" } else { "DESC" };

        // Build filter conditions based on category
        let (additional_conditions, extra_params): (String, Option<Vec<i16>>) = match filter_category {
            JobFilterCategory::All => {
                // No additional filtering
                (String::new(), None)
            }
            JobFilterCategory::RewardOnly => {
                // Filter for reward-eligible circuit types and completed status
                let conditions = r#"
                AND circuit_type = ANY($11::SMALLINT[])
                AND status = 'COMPLETED'
            "#
                .to_string();
                let params = Some(Self::get_reward_circuit_types());
                (conditions, params)
            }
        };

        // Build the complete query with additional conditions
        let query_str = format!(
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
                AND ($6::BIGINT IS NULL OR checkpoint_id >= $6)
                AND ($7::BIGINT IS NULL OR checkpoint_id <= $7)
                AND ($8::VARCHAR IS NULL OR public_key = $8)
            {}
            ORDER BY checkpoint_id {}, timestamp {}
            LIMIT $9 OFFSET $10
            "#,
            additional_conditions, order_direction, order_direction
        );

        // Execute query with appropriate bindings
        let rows = if let Some(circuit_types) = extra_params {
            // Query with extra circuit type filter parameter
            sqlx::query(&query_str)
                .bind(realm_id)
                .bind(status.map(|s| s.to_string()))
                .bind(source.map(|s| s.to_string()))
                .bind(topic)
                .bind(circuit_type)
                .bind(from_checkpoint_id)
                .bind(to_checkpoint_id)
                .bind(public_key)
                .bind(limit)
                .bind(offset)
                .bind(&circuit_types) // Bind the extra circuit types array
                .fetch_all(pool)
                .await?
        } else {
            // Query without extra parameter
            sqlx::query(&query_str)
                .bind(realm_id)
                .bind(status.map(|s| s.to_string()))
                .bind(source.map(|s| s.to_string()))
                .bind(topic)
                .bind(circuit_type)
                .bind(from_checkpoint_id)
                .bind(to_checkpoint_id)
                .bind(public_key)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
        };

        use psy_common::job::id::ProvingJobCircuitType;
        use sqlx::Row;

        use crate::models::job_id_from_json;

        // Create a default QProvingJobDataID in case of conversion failure
        let default_job_id = QProvingJobDataID::new_proof_job_id(0, 0, 0, ProvingJobCircuitType::Unknown, 0, 0);

        let events: Result<Vec<WorkerEvent>> = rows
            .into_iter()
            .map(|row| {
                let job_id_json: serde_json::Value = row.get("job_id");
                let parsed_job_id = job_id_from_json(job_id_json).unwrap_or(default_job_id.clone());

                let status = row
                    .get::<String, _>("status")
                    .parse()
                    .map_err(|e| anyhow::anyhow!("Failed to parse status: {}", e))?;

                let source = row
                    .get::<String, _>("source")
                    .parse()
                    .map_err(|e| anyhow::anyhow!("Failed to parse source: {}", e))?;

                Ok(WorkerEvent {
                    id: Some(row.get("id")),
                    realm_id: row.get("realm_id"),
                    public_key: row.get("public_key"),
                    status,
                    source,
                    job_id: parsed_job_id,
                    checkpoint_id: row.get("checkpoint_id"),
                    duration: row.get("duration"),
                    metadata: row.get("metadata"),
                    timestamp: row.get("timestamp"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            })
            .collect();

        events
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

    /// Get reward-eligible circuit types
    pub fn get_reward_circuit_types() -> Vec<i16> {
        vec![
            ProvingJobCircuitType::GUTAOnlyRegisterUsers.to_u8() as i16,                // 14
            ProvingJobCircuitType::GUTARegisterUsers.to_u8() as i16,                    // 12
            ProvingJobCircuitType::GUTATwoEndCap.to_u8() as i16,                        // 7
            ProvingJobCircuitType::GUTATwoGUTA.to_u8() as i16,                          // 8
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA.to_u8() as i16,              // 9
            ProvingJobCircuitType::GUTALeftGUTARightEndCap.to_u8() as i16,              // 10
            ProvingJobCircuitType::GUTASingleEndCap.to_u8() as i16,                     // 11
            ProvingJobCircuitType::GUTAVerifyToCap.to_u8() as i16,                      // 13
            ProvingJobCircuitType::GUTANoChange.to_u8() as i16,                         // 15
            ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade.to_u8() as i16,     // 55
            ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade.to_u8() as i16, // 56
        ]
    }
}
