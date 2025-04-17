use std::sync::Arc;

use anyhow::Result;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    server::ServerBuilder,
    types::error::{ErrorObject, INTERNAL_ERROR_CODE},
};
use plonky2::plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs};
use qed_core::{
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueEmitterAsyncImm,
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    },
};
use qed_data::{guta::end_cap_input::SubmitUserEndCapNonProofInput, qdata::user::QEDUserLeaf};
use qed_store::{config::store_config::QEDFelt, node::realm::QEDRealmStoreReaderAsync};
use tokio::sync::Mutex;
use tracing::{error, info};

use super::context::RealmEdgeContext;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

/// RPC interface definition for Realm Edge node
#[rpc(server, client, namespace = "realm")]
pub trait RealmEdgeRpc {
    /// Submit user end cap proof
    #[method(name = "submit_user_end_cap")]
    async fn submit_user_end_cap(
        &self,
        input: SubmitUserEndCapNonProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<bool>;

    /// Get contract height
    #[method(name = "get_contract_height")]
    async fn get_contract_height(&self, contract_id: u64) -> RpcResult<u8>;

    /// Get contract zero hash
    #[method(name = "get_contract_zero_hash")]
    async fn get_contract_zero_hash(&self, contract_id: u64) -> RpcResult<QHashOut<F>>;

    /// Get checkpoint information
    #[method(name = "get_checkpoint_info")]
    async fn get_checkpoint_info(&self) -> RpcResult<u64>;

    /// Check if a user id belongs to this realm
    #[method(name = "check_user_id_in_realm")]
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool>;

    /// Get user leaf data for a specific user
    #[method(name = "get_user_leaf_data")]
    async fn get_user_leaf_data(&self, user_id: u64) -> RpcResult<QEDUserLeaf<F>>;
}

/// Helper function to convert any error to a JSON-RPC error
fn to_rpc_error<T, E: std::fmt::Display>(context: &str, err: E) -> RpcResult<T> {
    error!("{}: {}", context, err);
    Err(ErrorObject::owned(
        INTERNAL_ERROR_CODE,
        format!("{}: {}", context, err),
        None::<()>,
    ))
}

/// RPC implementation for Realm Edge node
pub struct RealmEdgeRpcImpl<
    SR: QEDRealmStoreReaderAsync<F> + Send + Sync + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync + 'static,
    PS: QProofStoreAsyncImm
        + QProofStoreReaderAsync
        + QProofStoreWriterAsyncImm
        + Send
        + Sync
        + 'static,
> {
    pub ctx: Arc<Mutex<RealmEdgeContext<SR, DQ, PS>>>,
}

#[async_trait]
impl<SR, DQ, PS> RealmEdgeRpcServer for RealmEdgeRpcImpl<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Send + Sync + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync + 'static,
    PS: QProofStoreAsyncImm
        + QProofStoreReaderAsync
        + QProofStoreWriterAsyncImm
        + Send
        + Sync
        + 'static,
    RealmEdgeContext<SR, DQ, PS>: Send + 'static,
{
    /// Implementation of submit user End Cap proof RPC interface
    async fn submit_user_end_cap(
        &self,
        input: SubmitUserEndCapNonProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<bool> {
        let mut ctx = self.ctx.lock().await;

        ctx.handle_recv_end_cap_from_user(input, &proof)
            .await
            .map(|_| true)
            .map_err(|e| to_rpc_error::<bool, _>("Failed to process end cap", e).unwrap_err())
    }

    /// Implementation of get contract height RPC interface
    async fn get_contract_height(&self, contract_id: u64) -> RpcResult<u8> {
        let mut ctx = self.ctx.lock().await;

        ctx.get_contract_height(contract_id)
            .await
            .map_err(|e| to_rpc_error::<u8, _>("Failed to get contract height", e).unwrap_err())
    }

    /// Implementation of get contract zero hash RPC interface
    async fn get_contract_zero_hash(&self, contract_id: u64) -> RpcResult<QHashOut<F>> {
        let mut ctx = self.ctx.lock().await;

        ctx.get_contract_zero_hash(contract_id).await.map_err(|e| {
            to_rpc_error::<QHashOut<F>, _>("Failed to get contract zero hash", e).unwrap_err()
        })
    }

    /// Implementation of get checkpoint info RPC interface
    async fn get_checkpoint_info(&self) -> RpcResult<u64> {
        let ctx = self.ctx.lock().await;

        ctx.get_checkpoint_id_async()
            .await
            .map_err(|e| to_rpc_error::<u64, _>("Failed to get checkpoint info", e).unwrap_err())
    }

    /// Implementation of check user ID in realm RPC interface
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool> {
        let ctx = self.ctx.lock().await;

        Ok(ctx.includes_user_id(user_id))
    }

    /// Implementation of get user leaf data RPC interface
    async fn get_user_leaf_data(&self, user_id: u64) -> RpcResult<QEDUserLeaf<F>> {
        let ctx = self.ctx.lock().await;

        let checkpoint_id = ctx.get_checkpoint_id_async().await.map_err(|e| {
            to_rpc_error::<QEDUserLeaf<F>, _>("Failed to get checkpoint info", e).unwrap_err()
        })?;

        ctx.store_reader
            .get_user_leaf_data(checkpoint_id, user_id)
            .await
            .map_err(|e| {
                to_rpc_error::<QEDUserLeaf<F>, _>("Failed to get user leaf data", e).unwrap_err()
            })
    }
}

/// Start Realm Edge node RPC server
pub async fn start_realm_edge_rpc_server<
    SR: QEDRealmStoreReaderAsync<F> + Send + Sync + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync + 'static,
    PS: QProofStoreAsyncImm
        + QProofStoreReaderAsync
        + QProofStoreWriterAsyncImm
        + Send
        + Sync
        + 'static,
>(
    ctx: Arc<Mutex<RealmEdgeContext<SR, DQ, PS>>>,
    listen_addr: &str,
) -> Result<jsonrpsee::server::ServerHandle> {
    let server = ServerBuilder::default().build(listen_addr).await?;

    let rpc = RealmEdgeRpcImpl { ctx };

    let handle = server.start(rpc.into_rpc());
    info!("Realm Edge RPC server started on {}", listen_addr);
    Ok(handle)
}
