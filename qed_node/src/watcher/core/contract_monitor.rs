// contract_monitor.rs - Monitor contract deployments and report metadata
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use qed_data::qdata::contract_uuid::ContractUUID;
use qed_data::qdata::contract_metadata::ContractMetaData;
use qed_data::config::store_config::QEDFelt;
use qed_store::store::QEDStore;
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;
use crate::watcher::ApiClient;
use crate::common::utils::current_datetime;

const CONTRACT_CHECK_INTERVAL_SECS: u64 = 15;
const MAX_RETRY_ATTEMPTS: u32 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadataReport {
    pub contract_uuid: String,
    pub checkpoint_id: u64,
    pub contract_id: u64,
    pub deployer: String,
    pub function_whitelist_root: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct PendingContract {
    contract_uuid: ContractUUID,
    retry_count: u32,
    first_seen: chrono::DateTime<chrono::Utc>,
}

pub struct ContractMonitorService {
    qed_store: Arc<QEDStore>,
    api_client: Arc<ApiClient>,
    receiver: mpsc::UnboundedReceiver<ContractUUID>,
    pending_contracts: VecDeque<PendingContract>,
}

// Add a public handle type for integration with WatcherService
pub type ContractMonitorHandle = mpsc::UnboundedSender<ContractUUID>;

impl ContractMonitorService {
    pub fn new(
        qed_store: Arc<QEDStore>,
        api_client: Arc<ApiClient>,
    ) -> (Self, mpsc::UnboundedSender<ContractUUID>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let service = Self {
            qed_store,
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

    fn add_pending_contract(&mut self, contract_uuid: ContractUUID) {
        debug!("contract monitor service add pending contract, contract_uuid: {}", contract_uuid.to_string());
        // Check if already in queue
        if self.pending_contracts.iter().any(|p| p.contract_uuid == contract_uuid) {
            debug!("Contract {} already in monitoring queue", contract_uuid.to_string());
            return;
        }

        let pending = PendingContract {
            contract_uuid,
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

            match self.fetch_and_report_contract(&pending.contract_uuid).await {
                Ok(true) => {
                    info!(
                        "✅ Successfully reported metadata for contract {} (took {} attempts)",
                        pending.contract_uuid.to_string(),
                        pending.retry_count
                    );
                }
                Ok(false) => {
                    // Not found yet
                    if pending.retry_count >= MAX_RETRY_ATTEMPTS {
                        let elapsed = chrono::Utc::now() - pending.first_seen;
                        warn!(
                            "⚠️ Contract {} not found after {} attempts over {} seconds, removing from queue",
                            pending.contract_uuid.to_string(),
                            pending.retry_count,
                            elapsed.num_seconds()
                        );
                    } else {
                        debug!(
                            "Contract {} not found yet (attempt {}/{}), will retry",
                            pending.contract_uuid.to_string(),
                            pending.retry_count,
                            MAX_RETRY_ATTEMPTS
                        );
                        contracts_to_retry.push_back(pending);
                    }
                }
                Err(e) => {
                    error!(
                        "❌ Error checking contract {}: {}",
                        pending.contract_uuid.to_string(),
                        e
                    );

                    if pending.retry_count < MAX_RETRY_ATTEMPTS {
                        contracts_to_retry.push_back(pending);
                    } else {
                        warn!(
                            "⚠️ Giving up on contract {} after {} attempts",
                            pending.contract_uuid.to_string(),
                            pending.retry_count
                        );
                    }
                }
            }
        }

        // Re-add contracts that need to be retried
        self.pending_contracts = contracts_to_retry;
    }

    async fn fetch_and_report_contract(&self, contract_uuid: &ContractUUID) -> Result<bool> {
        // Try to fetch contract metadata from the database
        let contract_metadata = match self.qed_store
            .get_contract_metadata(*contract_uuid)
            .await
        {
            Ok(metadata) => metadata,
            Err(e) => {
                // If not found, it's not an error - the contract just hasn't been finalized yet
                debug!("Contract {} not yet available in database: {}", contract_uuid.to_string(), e);
                return Ok(false);
            }
        };

        debug!(
            "Found contract metadata for {}: checkpoint_id={}, contract_id={}",
            contract_uuid.to_string(),
            contract_metadata.checkpoint_id,
            contract_metadata.contract_id
        );

        // Convert to report format
        let report = ContractMetadataReport {
            contract_uuid: contract_uuid.to_string(),
            checkpoint_id: contract_metadata.checkpoint_id,
            contract_id: contract_metadata.contract_id,
            deployer: format!("{}", contract_metadata.deployer.to_string_le()),
            function_whitelist_root: format!("{}", contract_metadata.function_whitelist_root.to_string_le()),
            timestamp: current_datetime(),
        };

        // Send to API service
        self.send_contract_metadata_report(&report).await?;

        Ok(true)
    }

    async fn send_contract_metadata_report(&self, report: &ContractMetadataReport) -> Result<()> {
        let url = format!("{}/telemetry/contract", self.api_client.config.endpoint);
        let headers = self.api_client.get_headers("/telemetry/contract").await?;

        let response = self.api_client.client
            .post(&url)
            .headers(headers)
            .json(report)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to send contract metadata report: {}",
                response.status()
            );
        }

        info!(
            "📊 Contract metadata reported to API: contract_uuid={}, contract_id={}",
            report.contract_uuid,
            report.contract_id
        );

        Ok(())
    }
}