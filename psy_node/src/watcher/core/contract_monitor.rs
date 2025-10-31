// contract_monitor.rs - Monitor contract deployments and report metadata
use std::{collections::VecDeque, sync::Arc, time::Duration};

use anyhow::Result;
use psy_data::{
    config::store_config::PsyFelt,
};
use psy_store::{node::coordinator::PsyCoordinatorStoreReaderAsync, store::PsyStore};
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, time::interval};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    watcher::{events::UserContractMetadata, ApiClient},
};

const CONTRACT_CHECK_INTERVAL_SECS: u64 = 15;
const MAX_RETRY_ATTEMPTS: u32 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadataReport {
    pub contract_uuid: Uuid,
    pub checkpoint_id: u64,
    pub contract_id: u64,
    pub deployer: String,
    pub function_whitelist_root: String,
    pub metadata: serde_json::Value, // JSONB field storing complete UserContractMetadata
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct PendingContract {
    metadata: UserContractMetadata,
    retry_count: u32,
    first_seen: chrono::DateTime<chrono::Utc>,
}

pub struct ContractMonitorService {
    psy_store: Arc<PsyStore>,
    api_client: Arc<ApiClient>,
    receiver: mpsc::UnboundedReceiver<UserContractMetadata>,
    pending_contracts: VecDeque<PendingContract>,
}

// Add a public handle type for integration with WatcherService
pub type ContractMonitorHandle = mpsc::UnboundedSender<UserContractMetadata>;

impl ContractMonitorService {
    pub fn new(psy_store: Arc<PsyStore>, api_client: Arc<ApiClient>) -> (Self, mpsc::UnboundedSender<UserContractMetadata>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let service = Self {
            psy_store,
            api_client,
            receiver: rx,
            pending_contracts: VecDeque::new(),
        };

        (service, tx)
    }

    pub async fn run(mut self) -> Result<()> {
        info!("Starting contract monitor service");

        let mut check_interval = interval(Duration::from_secs(CONTRACT_CHECK_INTERVAL_SECS));

        loop {
            tokio::select! {
                // Receive new contract UUIDs from the channel
                Some(contract_uuid) = self.receiver.recv() => {
                    self.add_pending_contract(contract_uuid);
                }

                // Periodically check for contract metadata
                _ = check_interval.tick() => {
                    self.check_pending_contracts().await;
                }
            }
        }
    }

    fn add_pending_contract(&mut self, metadata: UserContractMetadata) {
        // Check if already in queue
        if self.pending_contracts.iter().any(|p| p.metadata.contract_uuid == metadata.contract_uuid) {
            debug!("Contract {} already in monitoring queue", metadata.contract_uuid.to_string());
            return;
        }

        let contract_uuid = metadata.contract_uuid;
        let pending = PendingContract {
            metadata,
            retry_count: 0,
            first_seen: chrono::Utc::now(),
        };

        self.pending_contracts.push_back(pending);
        info!(
            "Added contract {} to monitoring queue (total: {})",
            contract_uuid.to_string(),
            self.pending_contracts.len()
        );
    }
    async fn check_pending_contracts(&mut self) {
        if self.pending_contracts.is_empty() {
            return;
        }

        debug!("Checking {} pending contracts", self.pending_contracts.len());

        let mut contracts_to_retry = VecDeque::new();

        while let Some(mut pending) = self.pending_contracts.pop_front() {
            pending.retry_count += 1;

            match self.fetch_and_report_contract(&pending.metadata).await {
                Ok(true) => {
                    info!(
                        "✅ Successfully reported metadata for contract {} (took {} attempts)",
                        pending.metadata.contract_uuid.to_string(),
                        pending.retry_count
                    );
                }
                Ok(false) => {
                    // Not found yet
                    if pending.retry_count >= MAX_RETRY_ATTEMPTS {
                        let elapsed = chrono::Utc::now() - pending.first_seen;
                        warn!(
                            "⚠️ Contract {} not found after {} attempts over {} seconds, removing from queue",
                            pending.metadata.contract_uuid.to_string(),
                            pending.retry_count,
                            elapsed.num_seconds()
                        );
                    } else {
                        debug!(
                            "Contract {} not found yet (attempt {}/{}), will retry",
                            pending.metadata.contract_uuid.to_string(),
                            pending.retry_count,
                            MAX_RETRY_ATTEMPTS
                        );
                        contracts_to_retry.push_back(pending);
                    }
                }
                Err(e) => {
                    error!("❌ Error checking contract {}: {}", pending.metadata.contract_uuid.to_string(), e);

                    if pending.retry_count < MAX_RETRY_ATTEMPTS {
                        contracts_to_retry.push_back(pending);
                    } else {
                        warn!(
                            "⚠️ Giving up on contract {} after {} attempts",
                            pending.metadata.contract_uuid.to_string(),
                            pending.retry_count
                        );
                    }
                }
            }
        }

        // Re-add contracts that need to be retried
        self.pending_contracts = contracts_to_retry;
    }

    async fn fetch_and_report_contract(&self, user_metadata: &UserContractMetadata) -> Result<bool> {
        // Try to fetch contract metadata from the database
        let contract_metadata = match self.psy_store.get_contract_metadata(user_metadata.contract_uuid).await {
            Ok(metadata) => metadata,
            Err(e) => {
                // If not found, it's not an error - the contract just hasn't been finalized yet
                debug!(
                    "Contract {} not yet available in database: {}",
                    user_metadata.contract_uuid.to_string(),
                    e
                );
                return Ok(false);
            }
        };

        debug!(
            "Found contract metadata for {}: checkpoint_id={}, contract_id={}",
            user_metadata.contract_uuid.to_string(),
            contract_metadata.checkpoint_id,
            contract_metadata.contract_id
        );

        let contract_uuid = Uuid::from_u64_pair(user_metadata.contract_uuid.checkpoint_id, user_metadata.contract_uuid.uuid);

        // Serialize the complete UserContractMetadata to JSON
        let metadata_json = serde_json::to_value(user_metadata).map_err(|e| anyhow::anyhow!("Failed to serialize UserContractMetadata: {}", e))?;

        // Convert to report format
        let report = ContractMetadataReport {
            contract_uuid,
            checkpoint_id: contract_metadata.checkpoint_id,
            contract_id: contract_metadata.contract_id,
            deployer: format!("{}", contract_metadata.deployer.to_string_le()),
            function_whitelist_root: format!("{}", contract_metadata.function_whitelist_root.to_string_le()),
            metadata: metadata_json, // Store complete UserContractMetadata as JSONB
            timestamp: current_datetime(),
        };

        // Send to API service
        self.send_contract_metadata_report(&report).await?;

        Ok(true)
    }

    async fn send_contract_metadata_report(&self, report: &ContractMetadataReport) -> Result<()> {
        // Wrap the report in a payload structure to match the API's expected format
        #[derive(Serialize)]
        struct ContractTelemetryPayload {
            report: ContractMetadataReport,
        }

        let payload = ContractTelemetryPayload { report: report.clone() };

        // Use the existing send_telemetry_request method which includes JWT
        // authentication
        let response: serde_json::Value = self
            .api_client
            .send_telemetry_request("/telemetry/contract", &payload)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send contract metadata report: {}", e))?;

        info!(
            "📊 Contract metadata reported to API: contract_uuid={}, contract_id={}, response={}",
            report.contract_uuid,
            report.contract_id,
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| "<invalid json>".to_string())
        );

        Ok(())
    }
}
