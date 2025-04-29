use anyhow::Ok;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use tracing::{debug, info};
use qed_core::job::drain_queue::CheckpointDrainQueueEmitterAsyncImm;
use qed_core::job::id::{ProvingJobCircuitType, ProvingJobDataId};
use qed_core::job::traits::QProofStoreAsyncImm;
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput};
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_store::config::store_config::QEDFelt;
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use crate::context::with_temp_ctx_read_async;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

/// read latest checkpoint & proof, package into queue for Realm / RE
pub async fn handle_cp_sync<SR, DQ, PS>(_ctx: &CoordinatorEdgeContext<SR, DQ, PS>) -> anyhow::Result<()>
where
    SR: QEDCoordinatorStoreReaderAsync<F> + Send + Sync,
    PS: QProofStoreAsyncImm + Send + Sync,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync,
{
    //todo! maybe need some operations on sync_info

    // // 1) get the latest checkpoint id
    // let latest = ctx.store_reader.get_latest_l2_block_state().await?;
    // let checkpoint_id = latest.checkpoint_id;
    //
    // // 2) get the compact sync info
    // let _sync_info = ctx
    //     .store_reader
    //     .get_checkpoint_sync_info_compact(checkpoint_id)
    //     .await?;

    // 3) get the proof that is related to this checkpoint
    // let proof_opt = if let Some(pid) = sync_info.l2_block_state.proof_id {
    //     Some(ctx.proof_store.get_proof_by_id(pid).await?)
    // } else {
    //     None
    // };

    // info!("🟢 CE have synced checkpoint #{checkpoint_id} to downstream");
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
    
    tracing::info!("✅ processed realm job, checkpoint {}", realm_result.checkpoint_id);
    if realm_result.proof_id.circuit_type ==  ProvingJobCircuitType::GUTANoChange {
        tracing::info!(
            "✅ processed GUTANoChange from realm, checkpoint {}, no proof",
            realm_result.checkpoint_id
        );
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
