use plonky2::plonk::config::PoseidonGoldilocksConfig;
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

use serde_json::json;
use reqwest::Client;
use futures_util::future::join_all;
use anyhow::Result;
type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

/// read latest checkpoint & proof, package into queue for Realm / RE
pub async fn handle_cp_sync(latest_checkpoint_id: u64) -> anyhow::Result<()>
{
    info!("Handling checkpoint sync");
    // 1) get the latest sync info from the queue
    let checkpoint_sync_info =
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(&*ctx.store_reader, latest_checkpoint_id).await
    }).await?;
    info!("📥 Fetched checkpoint sync info from db: checkpoint_id={}",latest_checkpoint_id);

    // 2. package all the info into a CheckpointSyncInfo
    let sync_info = CheckpointSyncInfo {
        checkpoint_id: latest_checkpoint_id,
        description: None,
        source_coordinator_edge_id: None,
        sync_timestamp: 0,
        compact: checkpoint_sync_info,
    };
    // 3) broadcast the info to all realms
    broadcast_checkpoint(sync_info).await?;
    info!("✅ Broadcast checkpoint_id = {} to all realms", latest_checkpoint_id);

    Ok(())
}
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
                "method": "realm_receive_checkpoint_sync",
                "params": params,
                "id": 1,
            });

            let res = client.post(&rpc_url)
                .json(&payload)
                .send()
                .await?;

            if res.status().is_success() {
                tracing::info!("✅ Sent checkpoint sync to realm {}", realm_name);
                Ok(())
            } else {
                tracing::warn!("⚠️ Failed to send checkpoint sync to {}, status = {}", realm_name, res.status());
                Err(anyhow::anyhow!("Failed http"))
            }
        });

        tasks.push(task);
    }

    let results = futures_util::future::join_all(tasks).await;

    for result in results {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("❌ RPC call failed: {:?}", e),
            Err(e) => tracing::warn!("❌ Task spawn failed: {:?}", e),
        }
    }

    Ok(())
}
pub async fn process_realm_job<SR, DQ, PS>(
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
    with_temp_ctx_read_async::<_, _, _, C, D>(|temp_ctx| async move {
        temp_ctx.handle_recv_guta_from_realm(input, &realm_proof).await
    }).await?;


    info!(
        "✅ processed GUTA from realm, checkpoint {}",
        realm_result.checkpoint_id
    );
    Ok(())
}
