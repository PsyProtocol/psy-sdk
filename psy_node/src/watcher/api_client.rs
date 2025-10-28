use reqwest::Client;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, debug, warn};
use psy_api_services::handlers::{TelemetryPayload, TelemetryResponse};
use psy_api_services::models::{UserEvent, UserEventTxType, WorkerEvent, WorkerEventSource, WorkerEventStatus};
use psy_core::job::id::QProvingJobDataID;
use crate::watcher::events::{JobCompletedEvent, JobTimeoutEvent, UserRegistrationEvent, BackupProofEvent, BackupWitnessEvent, UserRegistrationMetadata, UserDeployContractMetadata, UserDeployContractEvent, UserGutaSubmissionEvent, JobStartedEvent, JobPendingEvent};
use crate::watcher::watcher::NodeType;
use crate::watcher::watcher_service::{current_datetime, current_timestamp, current_timestamp_mills};

pub struct ApiClient {
    client: Client,
    endpoint: String,
    node_id: String,
    node_type: NodeType,
    realm_id: Option<i64>,
}

impl ApiClient {
    pub fn new(
        endpoint: String,
        node_id: String,
        node_type: NodeType,
        realm_id: Option<i64>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            endpoint,
            node_id,
            node_type,
            realm_id,
        })
    }

    pub async fn send_user_registration(&self, event: UserRegistrationEvent) -> Result<()> {
        let api_event = UserEvent {
            user_id: format!("user_{}", event.public_key),
            public_key: event.public_key.clone(),
            tx_type: UserEventTxType::RegisterUser,
            metadata: Some(serde_json::to_value(UserRegistrationMetadata {
                registration_time: event.timestamp,
                node_id: self.node_id.clone(),
                node_type: format!("{:?}", self.node_type),
            })?),
            timestamp: current_datetime(),
            created_at: current_datetime(),
            updated_at: current_datetime(),
        };

        self.send_user_events(vec![api_event]).await?;
        debug!("User registration event sent for: {}", event.public_key);
        Ok(())
    }

    pub async fn send_contract_deployment(&self, event: UserDeployContractEvent) -> Result<()> {
        let api_event = UserEvent {
            user_id: format!("user_{}", event.deployer),
            public_key: event.deployer.clone(),
            tx_type: UserEventTxType::DeployContract,
            metadata: Some(serde_json::to_value(UserRegistrationMetadata {
                registration_time: event.timestamp,
                node_id: self.node_id.clone(),
                node_type: format!("{:?}", self.node_type),
            })?),
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
            realm_id: self.realm_id,
            public_key: None,
            status: WorkerEventStatus::Pending,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: None,
            metadata: Some(serde_json::json!({
                "event_type": "job_pending",
                "start_time": event.start_time,
                "node_id": self.node_id,
                "node_type": format!("{:?}", self.node_type),
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
            realm_id: self.realm_id,
            public_key: Some(event.worker_id.to_string()),
            status: WorkerEventStatus::Processing,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: None,
            metadata: Some(serde_json::json!({
                "event_type": "job_started",
                "start_time": event.start_time,
                "node_id": self.node_id,
                "node_type": format!("{:?}", self.node_type),
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
            realm_id: self.realm_id,
            public_key: event.worker_id.clone(),
            status: WorkerEventStatus::Completed,
            source: self.worker_source(),
            job_id: event.job_id.clone(),
            checkpoint_id: event.job_id.goal_id as i64,
            duration: Some(event.duration_ms as i64),
            metadata: Some(serde_json::json!({
                "start_time": event.start_time,
                "end_time": event.end_time,
                "node_id": self.node_id,
                "node_type": format!("{:?}", self.node_type),
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
            realm_id: self.realm_id,
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
                "node_id": self.node_id,
                "node_type": format!("{:?}", self.node_type),
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
            realm_id: self.realm_id,
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
                "node_id": self.node_id,
                "node_type": format!("{:?}", self.node_type),
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
            realm_id: self.realm_id,
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
                "node_id": self.node_id,
                "node_type": format!("{:?}", self.node_type),
            })),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.send_worker_events(vec![api_event]).await?;
        debug!("Witness backup event sent for job: {:?}", event.job_id);
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
            .post(&format!("{}/telemetry/events", self.endpoint))
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
        match self.node_type {
            NodeType::Coordinator => WorkerEventSource::Coordinator,
            NodeType::Realm => WorkerEventSource::Realm,
        }
    }
}