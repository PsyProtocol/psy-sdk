use std::cmp::max;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

const BLOCK_FINALITY_DEPTH: i64 = 5; // Wait for checkpoint_id - 5 for finality
const PROCESSING_INTERVAL: Duration = Duration::from_secs(30); // Run every 30 seconds
const BATCH_SIZE: usize = 1000; // Insert batch size

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventProcessorConfig {
    pub finality_depth: i64,
    pub processing_interval: Duration,
}

impl Default for EventProcessorConfig {
    fn default() -> Self {
        Self {
            finality_depth: BLOCK_FINALITY_DEPTH,
            processing_interval: PROCESSING_INTERVAL,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerEventForProcessing {
    pub id: uuid::Uuid,
    pub realm_id: Option<i64>,
    pub public_key: Option<String>,
    pub status: String,
    pub source: String,
    pub job_id: serde_json::Value,
    pub checkpoint_id: i64,
    pub duration: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub slot_id: Option<i64>, // Extracted from metadata
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub processed_checkpoints: Vec<i64>,
    pub total_events_processed: usize,
    pub events_by_status: HashMap<String, usize>,
    pub rollbacks_detected: Vec<RollbackInfo>,
    pub processing_time_ms: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackInfo {
    pub checkpoint_id: i64,
    pub conflicting_slots: Vec<i64>,
    pub selected_slot: i64,
    pub discarded_events_count: usize,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointProcessingState {
    pub checkpoint_id: i64,
    pub status: ProcessingStatus,
    pub events_count: usize,
    pub slot_id: Option<i64>,
    pub processed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    RollbackDetected,
}


impl ProcessingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProcessingStatus::Pending => "PENDING",
            ProcessingStatus::Processing => "PROCESSING",
            ProcessingStatus::Completed => "COMPLETED",
            ProcessingStatus::Failed => "FAILED",
            ProcessingStatus::RollbackDetected => "ROLLBACK_DETECTED",
        }
    }
}

pub struct WorkerEventProcessor {
    pool: PgPool,
    config: EventProcessorConfig,
    current_checkpoint: Arc<AtomicI64>,
    processing_states: Arc<RwLock<HashMap<i64, CheckpointProcessingState>>>,
}

impl WorkerEventProcessor {
    pub fn new(pool: PgPool) -> Self {
        Self::with_config(pool, EventProcessorConfig::default())
    }

    pub fn with_config(pool: PgPool, config: EventProcessorConfig) -> Self {
        Self {
            pool,
            config,
            current_checkpoint: Arc::new(AtomicI64::new(0)),
            processing_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the background processing task
    pub async fn start_processing_task(pool: PgPool, config: EventProcessorConfig) {
        let processor = Arc::new(Self::with_config(pool, config));

        info!("Starting Worker Event Processor background task");

        // Initialize current max checkpoint
        if let Err(e) = processor.update_checkpoint().await {
            error!("Failed to initialize max checkpoint: {}", e);
        }

        // Main processing loop
        let processor_clone = processor.clone();
        tokio::spawn(async move {
            let mut ticker = interval(processor_clone.config.processing_interval);

            loop {
                ticker.tick().await;

                match processor_clone.process_pending_events().await {
                    Ok(result) => {
                        if result.total_events_processed > 0 || !result.errors.is_empty() {
                            info!(
                                "Event processing completed: {} checkpoints, {} events, {} rollbacks, {} errors",
                                result.processed_checkpoints.len(),
                                result.total_events_processed,
                                result.rollbacks_detected.len(),
                                result.errors.len()
                            );

                            if !result.errors.is_empty() {
                                warn!("Processing errors: {:?}", result.errors);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Event processing failed: {}", e);
                    }
                }
            }
        });

        // Periodic max checkpoint update
        let processor_clone = processor.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(10));

            loop {
                ticker.tick().await;

                if let Err(e) = processor_clone.update_checkpoint().await {
                    error!("Failed to update max checkpoint: {}", e);
                }
            }
        });
    }

    /// Main processing logic
    pub async fn process_pending_events(&self) -> Result<ProcessingResult> {
        let start_time = std::time::Instant::now();
        let mut result = ProcessingResult {
            processed_checkpoints: Vec::new(),
            total_events_processed: 0,
            events_by_status: HashMap::new(),
            rollbacks_detected: Vec::new(),
            processing_time_ms: 0,
            errors: Vec::new(),
        };

        // Get current checkpoint and calculate finalized checkpoint
        let current_checkpoint = self.get_current_checkpoint().await;
        let finalized_checkpoint = current_checkpoint - self.config.finality_depth;

        if finalized_checkpoint <= 0 {
            debug!("No finalized checkpoints to process (current_checkpoint: {})", current_checkpoint);
            return Ok(result);
        }

        info!(
            "Processing events: current_checkpoint={}, finalized_up_to={}",
            current_checkpoint, finalized_checkpoint
        );

        // Get unprocessed checkpoints
        let checkpoints = self.get_unprocessed_checkpoints(finalized_checkpoint).await?;

        if checkpoints.is_empty() {
            debug!("No unprocessed checkpoints found");
            return Ok(result);
        }

        info!("Found {} unprocessed checkpoints to process", checkpoints.len());

        // Process each checkpoint
        for checkpoint_id in checkpoints {
            match self.process_checkpoint(checkpoint_id).await {
                Ok(checkpoint_result) => {
                    result.processed_checkpoints.push(checkpoint_id);
                    result.total_events_processed += checkpoint_result.events_processed;

                    // Update status counts
                    for (status, count) in checkpoint_result.events_by_status {
                        *result.events_by_status.entry(status).or_insert(0) += count;
                    }

                    // Add rollback info if detected
                    if let Some(rollback_info) = checkpoint_result.rollback_info {
                        result.rollbacks_detected.push(rollback_info);
                    }
                }
                Err(e) => {
                    error!("Failed to process checkpoint {}: {}", checkpoint_id, e);
                    result.errors.push(format!("Checkpoint {}: {}", checkpoint_id, e));

                    // Mark as failed
                    self.mark_checkpoint_failed(checkpoint_id, e.to_string()).await?;
                }
            }
        }

        result.processing_time_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Process a single checkpoint
    async fn process_checkpoint(&self, checkpoint_id: i64) -> Result<CheckpointResult> {
        info!("Processing checkpoint {}", checkpoint_id);

        // Mark as processing
        self.update_processing_state(checkpoint_id, ProcessingStatus::Processing).await?;

        // Fetch all worker events for this checkpoint
        let events = self.fetch_worker_events(checkpoint_id).await?;

        if events.is_empty() {
            info!("No events found for checkpoint {}", checkpoint_id);
            self.mark_checkpoint_completed(checkpoint_id, 0).await?;
            return Ok(CheckpointResult {
                checkpoint_id,
                events_processed: 0,
                events_by_status: HashMap::new(),
                rollback_info: None,
            });
        }

        // Group events by slot_id
        let events_by_slot = self.group_events_by_slot(&events);

        // Handle rollback detection
        let (selected_events, rollback_info) = if events_by_slot.len() > 1 {
            self.handle_rollback(checkpoint_id, events_by_slot).await?
        } else {
            // Single slot
            let slot_id = events_by_slot.keys().next().cloned().unwrap_or(0);
            debug!("No rollback detected for checkpoint {}: single slot {}", checkpoint_id, slot_id);
            let events = events_by_slot.into_iter().next().map(|(_, e)| e).unwrap_or_default();
            (events, None)
        };

        // Count events by status
        let mut events_by_status = HashMap::new();
        for event in &selected_events {
            *events_by_status.entry(event.status.clone()).or_insert(0) += 1;
        }

        // Insert into worker_job_events table
        let inserted_count = self.insert_job_events(checkpoint_id, &selected_events).await?;

        // Mark checkpoint as completed
        self.mark_checkpoint_completed(checkpoint_id, inserted_count).await?;

        info!(
            "Successfully processed checkpoint {}: {} events inserted",
            checkpoint_id, inserted_count
        );

        Ok(CheckpointResult {
            checkpoint_id,
            events_processed: inserted_count,
            events_by_status,
            rollback_info,
        })
    }

    pub async fn get_current_checkpoint(&self) -> i64 {
        self.current_checkpoint.load(Ordering::Relaxed)
    }

    pub fn set_current_checkpoint(&self, new_value: i64) -> bool {
        let mut current = self.current_checkpoint.load(Ordering::Relaxed);
        while new_value > current {
            match self.current_checkpoint.compare_exchange(
                current,
                new_value,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
        false
    }

    /// Handle rollback detection and resolution
    async fn handle_rollback(
        &self,
        checkpoint_id: i64,
        events_by_slot: HashMap<i64, Vec<WorkerEventForProcessing>>,
    ) -> Result<(Vec<WorkerEventForProcessing>, Option<RollbackInfo>)> {
        let slot_ids: Vec<i64> = events_by_slot.keys().cloned().collect();

        warn!(
            "Rollback detected at checkpoint {}: {} conflicting slots {:?}",
            checkpoint_id, slot_ids.len(), slot_ids
        );

        // Select the slot with the biggest ID (earliest)
        let selected_slot = *slot_ids.iter().max()
            .ok_or_else(|| anyhow::anyhow!("No slots found"))?;

        let mut discarded_count = 0;

        for (slot_id, events) in &events_by_slot {
            if *slot_id != selected_slot {
                discarded_count += events.len();
            }
        }

        let rollback_info = RollbackInfo {
            checkpoint_id,
            conflicting_slots: slot_ids,
            selected_slot,
            discarded_events_count: discarded_count,
            detected_at: Utc::now(),
        };

        // Record rollback detection
        self.record_rollback_detection(&rollback_info).await?;

        let selected_events = events_by_slot.into_iter()
            .find(|(slot_id, _)| *slot_id == selected_slot)
            .map(|(_, events)| events)
            .unwrap_or_default();

        Ok((selected_events, Some(rollback_info)))
    }

    /// Group events by slot_id
    fn group_events_by_slot(&self, events: &[WorkerEventForProcessing]) -> HashMap<i64, Vec<WorkerEventForProcessing>> {
        let mut grouped = HashMap::new();

        for event in events {
            let slot_id = event.slot_id.unwrap_or(0);
            grouped.entry(slot_id)
                .or_insert_with(Vec::new)
                .push(event.clone());
        }

        grouped
    }

    /// Get current maximum checkpoint_id from worker_events table
    async fn update_checkpoint(&self) -> Result<()> {
        let row = sqlx::query!(
            r#"
            SELECT MAX(checkpoint_id) as max_checkpoint
            FROM worker_events
            WHERE status = 'COMPLETED'
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        if let Some(max_checkpoint) = row.max_checkpoint {
            let current = self.get_current_checkpoint().await;

            if max_checkpoint > current {
                info!("Updating max checkpoint from {} to {}", current, max_checkpoint);
                let r = self.set_current_checkpoint(max_checkpoint);
                debug!("⬆️ Max checkpoint updated result: {}", r);
            } else {
                debug!("➡️  Max checkpoint remains at {}", current);
            }
        }

        Ok(())
    }

    /// Get unprocessed checkpoints that have reached finality
    async fn get_unprocessed_checkpoints(&self, up_to_checkpoint: i64) -> Result<Vec<i64>> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT we.checkpoint_id
            FROM worker_events we
            LEFT JOIN worker_job_events wje ON we.checkpoint_id = wje.checkpoint_id
            LEFT JOIN checkpoint_processing_status cps ON we.checkpoint_id = cps.checkpoint_id
            WHERE we.checkpoint_id <= $1
                AND we.status = 'COMPLETED'
                AND wje.checkpoint_id IS NULL
                AND (cps.status IS NULL OR cps.status != 'COMPLETED')
            ORDER BY we.checkpoint_id ASC
            "#,
            up_to_checkpoint,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.checkpoint_id).collect())
    }

    /// Fetch worker events for a specific checkpoint
    async fn fetch_worker_events(&self, checkpoint_id: i64) -> Result<Vec<WorkerEventForProcessing>> {
        let rows = sqlx::query_as!(
            WorkerEventForProcessing,
            r#"
            SELECT
                id, realm_id, public_key, status, source, job_id, checkpoint_id, duration, metadata,
                timestamp,
                CAST(COALESCE(
                    metadata->>'slot_id', '0'
                ) AS BIGINT) as "slot_id?"
            FROM worker_events
            WHERE checkpoint_id = $1
                AND status = 'COMPLETED'
            ORDER BY timestamp ASC
            "#,
            checkpoint_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
    /// Insert events into worker_job_events table
    /// Alternative implementation without ON CONFLICT clause
    async fn insert_job_events(&self, checkpoint_id: i64, events: &[WorkerEventForProcessing]) -> Result<usize> {
        let mut tx = self.pool.begin().await?;
        let mut inserted_count = 0;

        // First, get all existing job_ids for this checkpoint to avoid duplicates
        let existing_job_ids: Vec<String> = sqlx::query_scalar!(
            r#"
            SELECT job_id::text
            FROM worker_job_events
            WHERE checkpoint_id = $1
            "#,
            checkpoint_id
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .flatten()
        .collect();

        let existing_set: std::collections::HashSet<String> = existing_job_ids
            .into_iter()
            .map(|s| s.to_lowercase())
            .collect();

        for batch in events.chunks(BATCH_SIZE) {
            for event in batch {
                // Extract worker public key (required field)
                let worker_public_key = event.public_key.clone()
                    .unwrap_or_else(|| "unknown".to_string());

                // Extract topic and circuit_type from metadata or job_id
                let topic = event.metadata.as_ref()
                    .and_then(|m| m.get("topic"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i16;

                let circuit_type = event.metadata.as_ref()
                    .and_then(|m| m.get("circuit_type"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i16;

                // Check if this job_id already exists for this checkpoint
                let job_id_str = event.job_id.to_string().to_lowercase();
                if existing_set.contains(&job_id_str) {
                    debug!(
                    "Skipping duplicate job_id for checkpoint {}: {}",
                    checkpoint_id,
                    job_id_str
                );
                    continue;
                }

                // Insert the event
                match sqlx::query!(
                r#"
                INSERT INTO worker_job_events
                    (worker_public_key, checkpoint_id, job_id, topic, circuit_type,
                     duration, status, metadata, timestamp)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
                worker_public_key,
                checkpoint_id,
                event.job_id,
                topic,
                circuit_type,
                event.duration,
                event.status,
                event.metadata,
                event.timestamp
            )
            .execute(&mut *tx)
            .await
                {
                    Ok(_) => {
                        inserted_count += 1;
                    }
                    Err(e) => {
                        // Log error but continue processing other events
                        error!(
                        "Failed to insert worker_job_event for checkpoint {} job_id {}: {}",
                        checkpoint_id,
                        job_id_str,
                        e
                    );
                    }
                }
            }
        }

        tx.commit().await?;
        Ok(inserted_count)
    }

    /// Record rollback detection
    async fn record_rollback_detection(&self, rollback: &RollbackInfo) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO rollback_detections
                (checkpoint_id, conflicting_slots, selected_slot,
                 discarded_slots, affected_job_count, detection_time)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            rollback.checkpoint_id,
            &rollback.conflicting_slots,
            rollback.selected_slot,
            &rollback.conflicting_slots.iter()
                .filter(|&&s| s != rollback.selected_slot)
                .cloned()
                .collect::<Vec<_>>(),
            rollback.discarded_events_count as i32,
            rollback.detected_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update processing state for a checkpoint
    async fn update_processing_state(&self, checkpoint_id: i64, status: ProcessingStatus) -> Result<()> {
        let status_str = status.as_str();

        sqlx::query!(
            r#"
            INSERT INTO checkpoint_processing_status
                (checkpoint_id, status, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (checkpoint_id)
            DO UPDATE SET
                status = $2,
                updated_at = $3
            "#,
            checkpoint_id,
            status_str,
            Utc::now()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark checkpoint as completed
    async fn mark_checkpoint_completed(&self, checkpoint_id: i64, events_count: usize) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO checkpoint_processing_status
                (checkpoint_id, status, events_processed, processed_at, updated_at)
            VALUES ($1, 'COMPLETED', $2, $3, $3)
            ON CONFLICT (checkpoint_id)
            DO UPDATE SET
                status = 'COMPLETED',
                events_processed = $2,
                processed_at = $3,
                updated_at = $3
            "#,
            checkpoint_id,
            events_count as i32,
            Utc::now()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark checkpoint as failed
    async fn mark_checkpoint_failed(&self, checkpoint_id: i64, error: String) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO checkpoint_processing_status
                (checkpoint_id, status, error_message, updated_at)
            VALUES ($1, 'FAILED', $2, $3)
            ON CONFLICT (checkpoint_id)
            DO UPDATE SET
                status = 'FAILED',
                error_message = $2,
                updated_at = $3
            "#,
            checkpoint_id,
            error,
            Utc::now()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug)]
struct CheckpointResult {
    checkpoint_id: i64,
    events_processed: usize,
    events_by_status: HashMap<String, usize>,
    rollback_info: Option<RollbackInfo>,
}
