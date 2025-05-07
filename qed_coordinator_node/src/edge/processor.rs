use std::sync::atomic::Ordering;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use tracing::{debug, info};
use qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL;
use qed_core::job::drain_queue::CheckpointDrainQueueEmitterAsyncImm;
use qed_core::job::id::{ProvingJobCircuitType, ProvingJobDataId};
use qed_core::job::traits::QProofStoreAsyncImm;
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput};
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_store::config::store_config::QEDFelt;
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use crate::context::{with_ctx_read_async, with_temp_ctx_read_async, LATEST_CHECKPOINT_ID};
use crate::rpc::types::CheckpointSyncInfo;
use crate::CoordinatorEdgeQueueArgs;
use chrono::Utc;

use serde_json::json;
use reqwest::Client;
use futures_util::future::join_all;
use anyhow::Result;
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

const REALM_SYNC_INFO_METHOD: &str = "qed_sync_checkpoint";
pub async fn push_checkpoint_to_realm(realm_rpc_url: &str, sync_info: &CheckpointSyncInfo) -> Result<()> {
    let client = Client::new();

    let checkpoint_json = serde_json::to_value(sync_info)?;

    let payload = json!({
        "jsonrpc": "2.0",
        "method": REALM_SYNC_INFO_METHOD,
        "params": [checkpoint_json],
        "id": 1,
    });

    let res = client.post(realm_rpc_url)
        .json(&payload)
        .send()
        .await?;

    if res.status().is_success() {
        tracing::info!("✅ Successfully pushed checkpoint sync to realm: {}", realm_rpc_url);
        Ok(())
    } else {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        tracing::warn!("⚠️ Failed to push to realm: {}, status={}, body={}", realm_rpc_url, status, body);
        anyhow::bail!("Push to realm failed: status {}", status);
    }
}

pub fn build_checkpoint_sync_info(
    latest_checkpoint_id: u64,
    checkpoint_sync_info: QEDCheckpointSyncInfoCompact<QEDFelt>,
) -> CheckpointSyncInfo {
    CheckpointSyncInfo {
        latest_checkpoint_id,
        description: None,
        source_coordinator_edge_id: None,
        sync_timestamp: Utc::now().timestamp() as u64,
        compact: checkpoint_sync_info,
    }
}