use std::time::Duration;
use reqwest::Client;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::time::sleep;
use tracing::{info, debug, warn};
use qed_api_services::handlers::{TelemetryPayload, TelemetryResponse};
use qed_api_services::models::{UserEvent, UserEventTxType, WorkerEvent, WorkerEventSource, WorkerEventStatus};
use qed_core::job::id::QProvingJobDataID;
use crate::watcher::events::{JobCompletedEvent, JobTimeoutEvent, UserRegistrationEvent, BackupProofEvent, BackupWitnessEvent, UserRegistrationMetadata, UserContractMetadata, UserDeployContractEvent, UserGutaSubmissionEvent, JobStartedEvent, JobPendingEvent};
use crate::watcher::watcher::WatcherSourceNodeType;
use crate::watcher::watcher_service::{current_datetime, current_timestamp, current_timestamp_mills};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStatusReport {
    pub node_id: String,
    pub node_type: WatcherSourceNodeType,
    pub checkpoint_id: u64,
    pub block_height: u64,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointStats {
    pub checkpoint_id: i64,
    pub fees_collected: i64,
    pub user_ops_processed: i32,
    pub total_transactions: i32,
    pub slots_modified: i32,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEventPayload {
    pub worker_events: Vec<WorkerEvent>,
    pub user_events: Vec<UserEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}


const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_RETRY_COUNT: u32 = 3;
const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_BATCH_SIZE: usize = 100;

pub struct ApiClientConfig {
    pub endpoint: String,
    pub node_id: String,
    pub node_type: WatcherSourceNodeType,
    pub realm_id: Option<i64>,
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub enable_compression: bool,
}

impl ApiClientConfig {
    pub fn new(
        endpoint: String,
        node_id: String,
        node_type: WatcherSourceNodeType,
        realm_id: Option<i64>,
    ) -> Self {
        Self {
            endpoint,
            node_id,
            node_type,
            realm_id,
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_RETRY_COUNT,
            retry_delay: DEFAULT_RETRY_DELAY,
            enable_compression: true,
        }
    }
}


pub struct ApiClient {
    client: Client,
    config: ApiClientConfig,
}
impl ApiClient {
    pub fn new(
        endpoint: String,
        node_id: String,
        node_type: WatcherSourceNodeType,
        realm_id: Option<i64>,
    ) -> Result<Self> {
        let config = ApiClientConfig::new(endpoint, node_id, node_type, realm_id);
        Self::with_config(config)
    }

    pub fn with_config(config: ApiClientConfig) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10);

        if config.enable_compression {
            builder = builder.gzip(true).brotli(true);
        }

        let client = builder.build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    pub async fn send_user_registration(&self, event: UserRegistrationEvent) -> Result<()> {
        let api_event = UserEvent {
            user_id: format!("user_{}", &event.metadata.public_key),
            public_key: event.metadata.public_key.clone(),
            tx_type: UserEventTxType::RegisterUser,
            metadata: Some(serde_json::to_value(&event.metadata)?),
            timestamp: current_datetime(),
            created_at: current_datetime(),
            updated_at: current_datetime(),
        };

        self.send_user_events(vec![api_event]).await?;
        debug!("User registration event sent for: {}", &event.metadata.public_key);
        Ok(())
    }

    pub async fn send_contract_deployment(&self, event: UserDeployContractEvent) -> Result<()> {
        let api_event = UserEvent {
            user_id: format!("user_{}", event.deployer),
            public_key: event.deployer.clone(),
            tx_type: UserEventTxType::DeployContract,
            metadata: Some(serde_json::to_value(event.metadata.clone())?),
            timestamp: event.timestamp,
            created_at: current_datetime(),
            updated_at: current_datetime(),
        };

        self.send_user_events(vec![api_event]).await?;
        debug!("Contract deployment event sent for deployer: {}", event.deployer);
        Ok(())
    }

    pub async fn send_guta_submission(&self, event: UserGutaSubmissionEvent) -> Result<()> {
        let api_event = UserEvent {
            user_id: format!("realm_{}", event.realm_id),
            public_key: format!("realm_{}", event.realm_id),
            tx_type: UserEventTxType::Guta,
            metadata: Some(serde_json::to_value(event.metadata)?),
            timestamp: event.timestamp,
            created_at: current_datetime(),
            updated_at: current_datetime(),
        };

        self.send_user_events(vec![api_event]).await?;
        debug!("GUTA submission event sent for realm_id: {}", event.realm_id);
        Ok(())
    }
    pub async fn send_job_pending(&self, event: JobPendingEvent) -> Result<()> {
        let api_event = WorkerEvent {
            id: None,
            realm_id: self.config.realm_id,
            public_key: None,
            status: WorkerEventStatus::Pending,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: None,
            metadata: Some(serde_json::json!({
                "event_type": "job_pending",
                "start_time": event.start_time,
                "node_id": self.config.node_id,
                "node_type": format!("{:?}", self.config.node_type),
                "layer_id": event.job_id.task_index,
                "circuit_type": format!("{:?}", event.job_id.circuit_type),
            })),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.send_worker_events(vec![api_event]).await?;
        debug!("Job pending event sent for job: {:?}", event.job_id);
        Ok(())
    }

    pub async fn send_job_started(&self, event: JobStartedEvent) -> Result<()> {
        let api_event = WorkerEvent {
            id: None,
            realm_id: self.config.realm_id,
            public_key: Some(event.worker_id.to_string()),
            status: WorkerEventStatus::Processing,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: None,
            metadata: Some(serde_json::json!({
                "event_type": "job_started",
                "start_time": event.start_time,
                "node_id": self.config.node_id,
                "node_type": format!("{:?}", self.config.node_type),
                "layer_id": event.job_id.task_index,
                "circuit_type": format!("{:?}", event.job_id.circuit_type),
            })),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.send_worker_events(vec![api_event]).await?;
        debug!("Job started event sent for job: {:?}", event.job_id);
        Ok(())
    }

    pub async fn send_job_completed(&self, event: JobCompletedEvent) -> Result<()> {
        let api_event = WorkerEvent {
            id: None,
            realm_id: self.config.realm_id,
            public_key: event.worker_id.clone(),
            status: WorkerEventStatus::Completed,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: Some(event.duration_ms as i64),
            metadata: Some(serde_json::json!({
                "start_time": event.start_time,
                "end_time": event.end_time,
                "node_id": self.config.node_id,
                "node_type": format!("{:?}", self.config.node_type),
                "layer_id": event.job_id.task_index,
                "circuit_type": format!("{:?}", event.job_id.circuit_type),
            })),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.send_worker_events(vec![api_event]).await?;
        debug!("Job completed event sent for job: {:?}", event.job_id);
        Ok(())
    }

    pub async fn send_job_timeout(&self, event: JobTimeoutEvent) -> Result<()> {
        let api_event = WorkerEvent {
            id: None,
            realm_id: self.config.realm_id,
            public_key: event.worker_id.clone(),
            status: WorkerEventStatus::Failed,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: Some((event.timeout_time - event.start_time) as i64),
            metadata: Some(serde_json::json!({
                "timeout": true,
                "start_time": event.start_time,
                "timeout_time": event.timeout_time,
                "node_id": self.config.node_id,
                "node_type": format!("{:?}", self.config.node_type),
                "layer_id": event.job_id.task_index,
                "circuit_type": format!("{:?}", event.job_id.circuit_type),
            })),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.send_worker_events(vec![api_event]).await?;
        warn!("Job timeout event sent for job: {:?}", event.job_id);
        Ok(())
    }

    pub async fn send_proof_backup(&self, event: BackupProofEvent) -> Result<()> {
        let proof_hash = format!("{:x}", Sha256::digest(&event.proof_data));

        let api_event = WorkerEvent {
            id: None,
            realm_id: self.config.realm_id,
            public_key: None,
            status: WorkerEventStatus::Processing,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: None,
            metadata: Some(serde_json::json!({
                "backup_type": "proof",
                "proof_size": event.proof_data.len(),
                "proof_hash": proof_hash,
                "delete_after_blocks": event.delete_after_blocks,
                "backup_time": event.timestamp,
                "node_id": self.config.node_id,
                "node_type": format!("{:?}", self.config.node_type),
            })),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.send_worker_events(vec![api_event]).await?;
        debug!("Proof backup event sent for job: {:?}", event.job_id);
        Ok(())
    }

    pub async fn send_witness_backup(&self, event: BackupWitnessEvent) -> Result<()> {
        let witness_hash = format!("{:x}", Sha256::digest(&event.witness_data));

        let api_event = WorkerEvent {
            id: None,
            realm_id: self.config.realm_id,
            public_key: None,
            status: WorkerEventStatus::Processing,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: None,
            metadata: Some(serde_json::json!({
                "backup_type": "witness",
                "witness_size": event.witness_data.len(),
                "witness_hash": witness_hash,
                "delete_after_blocks": event.delete_after_blocks,
                "backup_time": event.timestamp,
                "node_id": self.config.node_id,
                "node_type": format!("{:?}", self.config.node_type),
            })),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.send_worker_events(vec![api_event]).await?;
        debug!("Witness backup event sent for job: {:?}", event.job_id);
        Ok(())
    }

    pub async fn report_block_status(&self, checkpoint_id: u64, block_height: u64) -> Result<()> {
        let report = BlockStatusReport {
            node_id: self.config.node_id.clone(),
            node_type: self.config.node_type,
            checkpoint_id,
            block_height,
            timestamp: current_datetime(),
            metadata: None,
        };

        self.send_with_retry(
            "POST",
            "/telemetry/block/status",
            Some(&report),
        ).await?;

        debug!(
            "Block status reported: checkpoint_id={}, block_height={}",
            checkpoint_id, block_height
        );
        Ok(())
    }

    pub async fn report_checkpoint_stats(&self, stats: CheckpointStats) -> Result<()> {
        let response: ApiResponse<serde_json::Value> = self.post_json(
            "/telemetry/checkpoint/stats",
            &stats,
        ).await?;

        if !response.success {
            return Err(anyhow::anyhow!(
                "Failed to report checkpoint stats: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        debug!("Checkpoint stats reported for checkpoint {}", stats.checkpoint_id);
        Ok(())
    }

    pub async fn send_batch_events(&self, worker_events: Vec<WorkerEvent>, user_events: Vec<UserEvent>) -> Result<()> {
        // Split into batches if needed
        for worker_batch in worker_events.chunks(MAX_BATCH_SIZE) {
            let payload = TelemetryPayload {
                worker_events: Some(worker_batch.to_vec()),
                user_events: None,
            };
            self.send_telemetry(&payload).await?;
        }

        for user_batch in user_events.chunks(MAX_BATCH_SIZE) {
            let payload = TelemetryPayload {
                worker_events: None,
                user_events: Some(user_batch.to_vec()),
            };
            self.send_telemetry(&payload).await?;
        }

        Ok(())
    }

    /// Sends worker events via telemetry
    async fn send_worker_events(&self, events: Vec<WorkerEvent>) -> Result<()> {
        let payload = TelemetryPayload {
            worker_events: Some(events),
            user_events: None,
        };
        self.send_telemetry(&payload).await
    }

    /// Sends user events via telemetry
    async fn send_user_events(&self, events: Vec<UserEvent>) -> Result<()> {
        let payload = TelemetryPayload {
            worker_events: None,
            user_events: Some(events),
        };
        self.send_telemetry(&payload).await
    }

    async fn send_telemetry(&self, payload: &TelemetryPayload) -> Result<()> {
        let response = self.client
            .post(&format!("{}/telemetry/events", self.config.endpoint))
            .json(payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to send telemetry to API service: {}",
                response.status()
            ));
        }

        let telemetry_response: TelemetryResponse = response.json().await?;
        debug!(
            "Telemetry sent successfully: {} events processed",
            telemetry_response.processed_count
        );

        Ok(())
    }

    fn worker_source(&self) -> WorkerEventSource {
        match self.config.node_type {
            WatcherSourceNodeType::Coordinator => WorkerEventSource::Coordinator,
            WatcherSourceNodeType::Realm => WorkerEventSource::Realm,
        }
    }

    async fn send_with_retry<T, R>(
        &self,
        method: &str,
        path: &str,
        body: Option<&T>,
    ) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.config.endpoint, path);
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.config.retry_delay * attempt;
                info!("Retrying request (attempt {}/{}), waiting {:?}", attempt, self.config.max_retries, delay);
                sleep(delay).await;
            }

            let request = match method {
                "GET" => self.client.get(&url),
                "POST" => self.client.post(&url),
                "PUT" => self.client.put(&url),
                "DELETE" => self.client.delete(&url),
                _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
            };

            let request = if let Some(body) = body {
                request.json(body)
            } else {
                request
            };

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return response.json::<R>().await
                            .context("Failed to parse response");
                    } else if !self.should_retry(response.status()) {
                        return Err(anyhow::anyhow!(
                            "Request failed with status: {}",
                            response.status()
                        ));
                    }
                    last_error = Some(format!("HTTP {}", response.status()));
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    if !self.is_retriable_error(&e) {
                        return Err(e.into());
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Request failed after {} attempts. Last error: {}",
            self.config.max_retries + 1,
            last_error.unwrap_or_else(|| "Unknown".to_string())
        ))
    }

    async fn post_json<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: for<'de> Deserialize<'de>,
    {
        self.send_with_retry("POST", path, Some(body)).await
    }

    async fn get_json<R>(&self, path: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        self.send_with_retry::<(), R>("GET", path, None).await
    }

    fn should_retry(&self, status: StatusCode) -> bool {
        matches!(
            status,
            StatusCode::REQUEST_TIMEOUT
                | StatusCode::TOO_MANY_REQUESTS
                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        )
    }

    fn is_retriable_error(&self, error: &reqwest::Error) -> bool {
        error.is_timeout() || error.is_connect() || error.is_request()
    }


    pub async fn health_check(&self) -> Result<bool> {
        match self.get_json::<serde_json::Value>("/health").await {
            Ok(response) => {
                if let Some(status) = response.get("status").and_then(|s| s.as_str()) {
                    Ok(status == "ok")
                } else {
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("Health check failed: {}", e);
                Ok(false)
            }
        }
    }

    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        self.get_json("/stats").await
    }

    pub async fn get_block_height(&self) -> Result<u64> {
        let stats: serde_json::Value = self.get_json("/stats").await?;
        stats.get("block_height")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Block height not found in stats"))
    }

}