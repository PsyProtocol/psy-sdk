use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use anyhow::{Context};
use chrono::{DateTime, Utc};
use http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use plonky2::field::types::PrimeField64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, debug, warn, error};
use qed_api_services::handlers::{TelemetryPayload, TelemetryResponse};
use qed_api_services::models::{CheckpointLeafStat, CheckpointLeavesRequest, CheckpointLeavesResponse, UserEvent, UserEventTxType, WorkerEvent, WorkerEventSource, WorkerEventStatus};
use qed_core::job::id::QProvingJobDataID;
use qed_data::config::store_config::QEDFelt;
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use crate::watcher::checkpoint_sender::CheckpointLeafWithId;
use crate::watcher::events::{JobCompletedEvent, JobPendingEvent, JobStartedEvent, JobTimeoutEvent, UserContractMetadata, UserDeployContractEvent, UserEndcapSubmissionEvent, UserGutaSubmissionEvent, UserRegistrationEvent, UserRegistrationMetadata};
use crate::watcher::timeout_watcher::WatcherSourceNodeType;
use anyhow::Result;
use crate::common::utils::current_datetime;
use crate::watcher::constant::{DEFAULT_HTTP_TIMEOUT, DEFAULT_RETRY_DELAY, JWT_EXPIRATION_HOURS, MAX_BATCH_SIZE, MAX_RETRY_ATTEMPTS};

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    service: Option<String>,
    iat: i64,
    exp: i64,
}

pub struct ApiClientConfig {
    pub endpoint: String,
    pub node_id: String,
    pub node_type: WatcherSourceNodeType,
    pub realm_id: Option<i64>,
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub enable_compression: bool,
    pub jwt_secret: Option<String>,  // JWT secret only for telemetry endpoints
}


impl ApiClientConfig {
    pub fn new(
        endpoint: String,
        node_id: String,
        node_type: WatcherSourceNodeType,
        realm_id: Option<i64>,
    ) -> Self {
        // Try to load JWT secret from environment
        dotenv::dotenv().ok();
        let jwt_secret = std::env::var("JWT_SECRET").ok();

        if jwt_secret.is_none() {
            warn!("JWT_SECRET not found in environment. Telemetry endpoints may fail authentication.");
        }

        Self {
            endpoint,
            node_id,
            node_type,
            realm_id,
            timeout: DEFAULT_HTTP_TIMEOUT,
            max_retries: MAX_RETRY_ATTEMPTS,
            retry_delay: DEFAULT_RETRY_DELAY,
            enable_compression: true,
            jwt_secret,
        }
    }

    pub fn with_jwt_secret(mut self, secret: String) -> Self {
        self.jwt_secret = Some(secret);
        self
    }
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

pub struct ApiClient {
    pub(crate) client: Client,
    pub(crate) config: ApiClientConfig,
    jwt_token: Arc<RwLock<Option<String>>>,  // Cached JWT token with interior mutability
    token_expiry: Arc<RwLock<Option<DateTime<Utc>>>>,  // Token expiration time with interior mutability
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

        let client = builder.build().context("Failed to create HTTP client")?;

        let api_client = Self {
            client,
            config,
            jwt_token: Arc::new(RwLock::new(None)),
            token_expiry: Arc::new(RwLock::new(None)),
        };

        Ok(api_client)

    }

    /// Generate a new JWT token using the shared secret
    fn generate_jwt_token(&self) -> Result<String> {
        let jwt_secret = self.config.jwt_secret.as_ref()
            .ok_or_else(|| anyhow::anyhow!("JWT_SECRET not configured"))?;

        let now = Utc::now();
        let exp = now + chrono::Duration::hours(JWT_EXPIRATION_HOURS);

        let claims = JwtClaims {
            sub: format!("watcher-{}", self.config.node_id),
            service: Some(format!("watcher-{:?}", self.config.node_type)),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes()),
        ).context("Failed to generate JWT token")?;

        debug!(
            "Generated JWT token for watcher-{} (expires at {})",
            self.config.node_id,
            exp
        );

        Ok(token)
    }

    /// Refresh JWT token if expired or not yet generated
    async fn refresh_jwt_token(&self) -> Result<()> {
        // Check if we need to refresh (no token or expired)
        let needs_refresh = {
            let expiry = self.token_expiry.read().await;
            if let Some(exp) = *expiry {
                Utc::now() >= exp - chrono::Duration::minutes(5)  // Refresh 5 minutes before expiry
            } else {
                true
            }
        };

        if needs_refresh {
            let token = self.generate_jwt_token()?;
            let expiry = Utc::now() + chrono::Duration::hours(JWT_EXPIRATION_HOURS);
            *self.jwt_token.write().await = Some(token);
            *self.token_expiry.write().await = Some(expiry);
            info!("JWT token refreshed for watcher-{}", self.config.node_id);
        }

        Ok(())
    }

    /// Get headers for a request, including JWT for telemetry endpoints
    pub(crate) async fn get_headers(&self, path: &str) -> Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);

        // Add JWT token for telemetry endpoints only
        if path.starts_with("/telemetry/") {
            self.refresh_jwt_token().await?;

            let token = self.jwt_token.read().await;
            if let Some(token_str) = &*token {
                headers.insert(
                    "Authorization",
                    format!("Bearer {}", token_str).parse()
                        .context("Failed to create Authorization header")?
                );
            } else {
                warn!("No JWT token available for telemetry endpoint: {}", path);
            }
        }

        Ok(headers)
    }

    // ========================================================================
    // User Event Handlers
    // ========================================================================
    pub async fn send_user_registration(&self, event: UserRegistrationEvent) -> Result<()> {
        let short_key = event
            .metadata
            .public_key
            .get(..8)
            .unwrap_or(&event.metadata.public_key);

        let api_event = UserEvent {
            user_id: format!("user_{}", short_key),
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
        let short_key = event
            .deployer
            .get(..8)
            .unwrap_or(&event.deployer);

        let api_event = UserEvent {
            user_id: format!("deployer_{}", short_key),
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
    pub async fn send_endcap_submission(&self, event: UserEndcapSubmissionEvent) -> Result<()> {
        let api_event = UserEvent {
            user_id: format!("user_{}", event.user_id),
            public_key: format!("{}", event.metadata.new_user_leaf.public_key),
            tx_type: UserEventTxType::UserEndcap,
            metadata: Some(serde_json::to_value(event.metadata)?),
            timestamp: event.timestamp,
            created_at: current_datetime(),
            updated_at: current_datetime(),
        };

        self.send_user_events(vec![api_event]).await?;
        debug!("UserEndcap submission event sent for realm_id: {}, user_id: {}", event.realm_id, event.user_id);
        Ok(())
    }
    pub async fn send_job_pending(&self, event: JobPendingEvent) -> Result<()> {
        let worker_event = self.build_worker_event(
            None,
            WorkerEventStatus::Pending,
            &event.job_id,
            None,
            serde_json::json!({
                "event_type": "job_pending",
                "start_time": event.start_time,
                "layer_id": event.job_id.task_index,
            }),
        );

        self.send_worker_events(vec![worker_event]).await
    }

    pub async fn send_job_started(&self, event: JobStartedEvent) -> Result<()> {
        let worker_event = self.build_worker_event(
            Some(event.worker_id.to_string()),
            WorkerEventStatus::Processing,
            &event.job_id,
            None,
            serde_json::json!({
                "event_type": "job_started",
                "start_time": event.start_time,
                "layer_id": event.job_id.task_index,
            }),
        );

        self.send_worker_events(vec![worker_event]).await
    }

    pub async fn send_job_completed(&self, event: JobCompletedEvent) -> Result<()> {
        let worker_event = self.build_worker_event(
            event.worker_id.clone(),
            WorkerEventStatus::Completed,
            &event.job_id,
            Some(event.duration_ms as i64),
            serde_json::json!({
                "event_type": "job_completed",
                "start_time": event.start_time,
                "end_time": event.end_time,
                "layer_id": event.job_id.task_index,
            }),
        );

        self.send_worker_events(vec![worker_event]).await
    }

    pub async fn send_job_timeout(&self, event: JobTimeoutEvent) -> Result<()> {
        let worker_event = self.build_worker_event(
            event.worker_id.clone(),
            WorkerEventStatus::Failed,
            &event.job_id,
            Some((event.timeout_time - event.start_time) as i64),
            serde_json::json!({
                "event_type": "job_timeout",
                "timeout": true,
                "start_time": event.start_time,
                "timeout_time": event.timeout_time,
                "layer_id": event.job_id.task_index,
            }),
        );

        self.send_worker_events(vec![worker_event]).await
    }


    fn build_worker_event(
        &self,
        public_key: Option<String>,
        status: WorkerEventStatus,
        job_id: &QProvingJobDataID,
        duration: Option<i64>,
        mut metadata: serde_json::Value,
    ) -> WorkerEvent {
        // Add common metadata fields
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("node_id".to_string(), serde_json::json!(self.config.node_id));
            obj.insert("node_type".to_string(), serde_json::json!(format!("{:?}", self.config.node_type)));
            obj.insert("circuit_type".to_string(), serde_json::json!(format!("{:?}", job_id.circuit_type)));
        }

        WorkerEvent {
            id: None,
            realm_id: self.config.realm_id,
            public_key,
            status,
            source: self.worker_source(),
            job_id: job_id.clone(),
            checkpoint_id: job_id.goal_id as i64,
            duration,
            metadata: Some(metadata),
            timestamp: current_datetime(),
            created_at: current_datetime(),
            updated_at: current_datetime(),
        }
    }

    pub async fn send_checkpoint_leaves(&self, checkpoint_leaves: Vec<CheckpointLeafWithId>) -> Result<()> {

        for batch in checkpoint_leaves.chunks(MAX_BATCH_SIZE) {

            let leaves: Vec<CheckpointLeafStat> = batch
                .iter()
                .map(|leaf| {
                    let stats = &leaf.checkpoint_leaf.stats;
                    CheckpointLeafStat {
                        checkpoint_id: leaf.checkpoint_id as i64,
                        fees_collected: stats.fees_collected.to_canonical_u64() as i64,
                        user_ops_processed: stats.user_ops_processed.to_canonical_u64() as i64,
                        total_transactions: stats.total_transactions.to_canonical_u64() as i64,
                        slots_modified: stats.slots_modified.to_canonical_u64() as i64,
                        metadata: serde_json::to_value(leaf).ok(),
                        timestamp: current_datetime(),
                    }
                })
                .collect();

            let request = CheckpointLeavesRequest {
                leaves,
                timestamp: current_datetime(),
            };

            self.send_telemetry_request::<CheckpointLeavesRequest, CheckpointLeavesResponse>("/telemetry/checkpoint/leaves", &request).await?;
        }

        Ok(())
    }


    async fn send_worker_events(&self, events: Vec<WorkerEvent>) -> Result<()> {
        let payload = TelemetryPayload {
            worker_events: Some(events),
            user_events: None,
        };
        self.send_telemetry(&payload).await
    }

    async fn send_user_events(&self, events: Vec<UserEvent>) -> Result<()> {
        let payload = TelemetryPayload {
            worker_events: None,
            user_events: Some(events),
        };
        self.send_telemetry(&payload).await
    }

    pub async fn send_batch_events(&self, worker_events: Vec<WorkerEvent>, user_events: Vec<UserEvent>) -> Result<()> {
        for batch in worker_events.chunks(MAX_BATCH_SIZE) {
            self.send_worker_events(batch.to_vec()).await?;
        }
        for batch in user_events.chunks(MAX_BATCH_SIZE) {
            self.send_user_events(batch.to_vec()).await?;
        }
        Ok(())
    }


    async fn send_telemetry(&self, payload: &TelemetryPayload) -> Result<()> {
        let _response: TelemetryResponse = self
            .send_telemetry_request("/telemetry/events", payload)
            .await?;

        // debug!("Telemetry sent: {} events processed", response.processed_count);
        Ok(())
    }

    // Generic telemetry request handler
    pub(crate) async fn send_telemetry_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        payload: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.config.endpoint, path);
        let headers = self.get_headers(path).await?;

        let response = self.client
            .post(&url)
            .headers(headers)
            .json(payload)
            .send()
            .await?;

        debug!("response: {:?}", response);
        match response.status() {
            StatusCode::UNAUTHORIZED => {
                error!("JWT authentication failed");
                Err(anyhow::anyhow!("Authentication failed - invalid JWT token"))
            }
            status if !status.is_success() => {
                Err(anyhow::anyhow!("Request failed with status: {}", status))
            }
            _ => response.json().await.context("Failed to parse response"),
        }
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
                Ok(response) if response.status().is_success() => {
                    return response.json().await.context("Failed to parse response");
                }
                Ok(response) if !self.should_retry(response.status()) => {
                    return Err(anyhow::anyhow!("Request failed: {}", response.status()));
                }
                Ok(response) => {
                    last_error = Some(format!("HTTP {}", response.status()));
                }
                Err(e) if !self.is_retriable_error(&e) => {
                    return Err(e.into());
                }
                Err(e) => {
                    last_error = Some(e.to_string());
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
        match self.send_with_retry::<(), serde_json::Value>("GET", "/health", None).await {
            Ok(response) => Ok(response.get("status")
                .and_then(|s| s.as_str())
                .map_or(false, |s| s == "ok")),
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