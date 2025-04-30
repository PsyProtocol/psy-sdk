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
use crate::context::{with_ctx_read_async, with_temp_ctx_read_async, GLOBAL_REALM_REGISTRY};
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

// /// read latest checkpoint & proof, package into queue for Realm / RE
// pub async fn handle_cp_sync(args: CoordinatorEdgeQueueArgs, latest_checkpoint_id: u64) -> anyhow::Result<()>
// {
//     info!("Handling checkpoint sync");
//     // 1) get the latest sync info from the queue
//     let checkpoint_sync_info =
//         with_temp_ctx_read_async::<_,_,_,C,D>(args,|ctx| async move {
//         QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(&*ctx.store_reader, latest_checkpoint_id).await
//     }).await?;
//     info!("📥 Fetched checkpoint sync info from db: checkpoint_id={}",latest_checkpoint_id);
//
//     // 2. package all the info into a CheckpointSyncInfo
//     let sync_info = build_checkpoint_sync_info(
//         latest_checkpoint_id,
//         checkpoint_sync_info,
//     );
//     // 3) broadcast the info to all realms
//     broadcast_checkpoint(sync_info).await?;
//     info!("✅ Broadcast checkpoint {} sync info to all realms", latest_checkpoint_id);
//
//     Ok(())
// }
pub async fn broadcast_checkpoint(sync_info: CheckpointSyncInfo) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let checkpoint_json = serde_json::to_value(&sync_info)?;

    let registry = GLOBAL_REALM_REGISTRY.read().await;
    let endpoints: Vec<_> = registry.realms.values().cloned().collect();
    drop(registry);// drop the read lock

    let mut tasks = Vec::new();

    for realm in endpoints {
        let client = client.clone();
        let rpc_url = realm.rpc_url.clone();
        let realm_name = realm.name.clone();
        let params = checkpoint_json.clone();

        let task = tokio::spawn(async move {
            let payload = serde_json::json!({
                "jsonrpc": "2.0",
                "method": REALM_SYNC_INFO_METHOD,
                "params": [params],
                "id": 1,
            });

            let res = client.post(&rpc_url)
                .json(&payload)
                .send()
                .await;

            (realm_name, rpc_url, res)
        });

        tasks.push(task);
    }

    let results = futures_util::future::join_all(tasks).await;

    let mut success_count = 0;
    let mut total_count = 0;
    let mut failed_realms = Vec::new();

    for result in results {
        total_count += 1;

        match result {
            Ok((realm_name, rpc_url, Ok(res))) => {
                if res.status().is_success() {
                    match res.json::<serde_json::Value>().await {
                        Ok(json) => {
                            if json.get("error").is_none() {
                                tracing::info!("✅ Successfully sent and confirmed checkpoint sync to realm {}", realm_name);
                                success_count += 1;
                            } else {
                                tracing::warn!("⚠️ Realm {} RPC returned error: {:?}", realm_name, json.get("error"));
                                failed_realms.push((realm_name, rpc_url, "RPC error field present".to_string()));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("⚠️ Failed to parse RPC response from realm {}: {:?}", realm_name, e);
                            failed_realms.push((realm_name, rpc_url, format!("Invalid RPC response: {:?}", e)));
                        }
                    }
                } else {
                    failed_realms.push((realm_name, rpc_url, format!("HTTP status {}", res.status())));
                }
            }
            Ok((realm_name, rpc_url, Err(e))) => {
                failed_realms.push((realm_name, rpc_url, e.to_string()));
            }
            Err(e) => {
                failed_realms.push(("unknown".to_string(), "unknown".to_string(), format!("Join error: {:?}", e)));
            }
        }
    }
    tracing::info!("📈 Broadcast result: {}/{} realms succeeded", success_count, total_count);

    if !failed_realms.is_empty() {
        tracing::warn!("❗ Failed realms:");
        for (name, url, reason) in failed_realms {
            tracing::warn!(" - {} ({}) => {}", name, url, reason);
        }
    }

    Ok(())
}

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


pub async fn fetch_and_push_checkpoint_to_realm<F, C, const D: usize>(
    args: CoordinatorEdgeQueueArgs,
    latest_checkpoint_id: u64,
    realm_rpc_url: &str,
) -> anyhow::Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    info!("🔍 Handling checkpoint sync to {}", realm_rpc_url);

    // 1) got QEDCheckpointSyncInfoCompact
    let checkpoint_sync_info = with_temp_ctx_read_async::<_, _, _, C, D>(args,|ctx| async move {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(&*ctx.store_reader, latest_checkpoint_id).await
    })
        .await?;

    info!("📥 Fetched checkpoint sync info: checkpoint_id={}", latest_checkpoint_id);

    // 2) CheckpointSyncInfo
    let sync_info = build_checkpoint_sync_info(latest_checkpoint_id, checkpoint_sync_info);

    // 3) push
    push_checkpoint_to_realm(realm_rpc_url, &sync_info).await?;

    Ok(())
}

pub async fn process_realm_job<SR, DQ, PS>(
    args: CoordinatorEdgeQueueArgs,
    ctx: &CoordinatorEdgeContext<SR, DQ, PS>,
    job_info: ProvingJobDataId,
) -> anyhow::Result<()>
where
    SR: QEDCoordinatorStoreReaderAsync<QEDFelt>,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
{
    info!("Processing realm job");
    info!("job_info: {:?}", job_info);
    // 1) got the bytes from job_info.job_id
    let bytes = ctx.proof_store.get_bytes_by_id(job_info.job_id).await?;
    let preview_len = bytes.len().min(100);
    let hex_preview = hex::encode(&bytes[..preview_len]);
    debug!("❗ the bytes from job_info.job_id: len = {}, head[0..{}] = {}",
        bytes.len(),
        preview_len,
        hex_preview
    );

    let realm_result: GUTARealmCheckpointResult<QEDFelt>  =
        bincode::deserialize(&bytes)
            .map_err(|e| anyhow::anyhow!("failed to deserialize realm_result: {:?}", e))?;

    //if circuit type is GUTANoChange, disable the proof
    if realm_result.proof_id.circuit_type == ProvingJobCircuitType::GUTANoChange {
        info!("⚠️ GUTANoChange proof, disabling it");
        return Ok(());
    }
    // 2) get the proof by id
    let realm_proof = ctx
        .proof_store
        .get_proof_by_id(realm_result.proof_id.get_output_id())
        .await?;

    // 3) call the context to handle the proof
    let input = SubmitGUTARealmResultAPINoProofInput {
        realm_id: 0, // TODO: replace with the real realm id
        checkpoint_id: realm_result.checkpoint_id,
        guta_stats: realm_result.guta_stats,
        top_line_proof: realm_result.top_line_proof,
        checkpoint_tree_root: realm_result.checkpoint_tree_root,
        circuit_type: realm_result.proof_id.circuit_type,
    };

    // ctx.handle_recv_guta_from_realm(input, &realm_proof).await?;
    with_temp_ctx_read_async::<_, _, _, C, D>(args,|temp_ctx| async move {
        temp_ctx.handle_recv_guta_from_realm(input, &realm_proof).await
    }).await?;


    info!(
        "✅ processed GUTA from realm, checkpoint {}",
        realm_result.checkpoint_id
    );
    Ok(())
}


pub fn build_checkpoint_sync_info(
    latest_checkpoint_id: u64,
    checkpoint_sync_info: QEDCheckpointSyncInfoCompact<QEDFelt>,
) -> CheckpointSyncInfo {
    CheckpointSyncInfo {
        checkpoint_id: latest_checkpoint_id,
        description: None,
        source_coordinator_edge_id: None,
        sync_timestamp: Utc::now().timestamp() as u64,
        compact: checkpoint_sync_info,
    }
}