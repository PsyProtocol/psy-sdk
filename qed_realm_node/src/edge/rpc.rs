use std::sync::Arc;

use super::{context::RealmEdgeContext, error::RpcError};
use crate::edge::request::QSubmitEndCapRPCRequest;
use crate::F;
use anyhow::Result;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    server::ServerBuilder,
    types::error::{ErrorObject, INTERNAL_ERROR_CODE},
};
use qed_core::data::qhashout::QHashOut;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueEmitterAsyncImm,
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
};
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use qed_data::qdata::contract::{ContractCodeDefinition, QEDContractLeaf};
use qed_data::qdata::{checkpoint::QEDL2BlockState, user::QEDUserLeaf};
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::store::imm::cmd::{
    QSRCmdGetCheckpointLeafData, QSRCmdGetContractCodeDefinition, QSRCmdGetContractLeafData,
    QSRCmdGetL2BlockState, QSRCmdGetUserLeafData, QSRHashCmd, QSRMerkleCmd,
};
use qed_store::store::imm::cmd_processor::{
    QEDReadCommandBatchInput, QEDReadCommandBatchOutput, QEDReadCommandProcessorSync,
};
use tracing::{error, info};

/// RPC interface definition for Realm Edge node
#[rpc(server, client, namespace = "qed")]
pub trait RealmEdgeRpc {
    /// Check if a user id belongs to this realm
    #[method(name = "check_user_id_in_realm")]
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool>;

    /// Submit user end cap proof
    #[method(name = "submit_user_end_cap")]
    async fn submit_user_end_cap(&self, req: QSubmitEndCapRPCRequest<F>) -> RpcResult<bool>;

    /// Submit a token transfer
    // #[method(name = "token_transfer")]
    // async fn token_transfer(&self, input: QTokenTransferRPCRequest) -> RpcResult<()>;

    /// get a batch of read commands
    #[method(name = "batch")]
    async fn get_batch(
        &self,
        input: QEDReadCommandBatchInput,
    ) -> RpcResult<QEDReadCommandBatchOutput<F>>;

    /// Get hash of a given input
    #[method(name = "get_hash")]
    async fn get_hash(&self, input: QSRHashCmd) -> RpcResult<QHashOut<F>>;

    /// Get merkle proof of a given input
    #[method(name = "get_merkle_proof")]
    async fn get_merkle_proof(
        &self,
        input: QSRMerkleCmd,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    /// Get user leaf data for a specific user
    #[method(name = "get_user_leaf")]
    async fn get_user_leaf(&self, input: QSRCmdGetUserLeafData) -> RpcResult<QEDUserLeaf<F>>;

    /// Get contract leaf data for a specific contract
    #[method(name = "get_contract_leaf")]
    async fn get_contract_leaf(
        &self,
        input: QSRCmdGetContractLeafData,
    ) -> RpcResult<QEDContractLeaf<F>>;

    /// Get contract code for a specific contract
    #[method(name = "get_contract_code")]
    async fn get_contract_code(
        &self,
        input: QSRCmdGetContractCodeDefinition,
    ) -> RpcResult<ContractCodeDefinition>;

    /// Get checkpoint leaf data for a specific checkpoint
    #[method(name = "get_checkpoint_leaf")]
    async fn get_checkpoint_leaf(
        &self,
        input: QSRCmdGetCheckpointLeafData,
    ) -> RpcResult<QEDCheckpointLeaf<F>>;

    /// Get L2 block state for a specific L2 block
    #[method(name = "get_l2_block_state")]
    async fn get_l2_block_state(&self, input: QSRCmdGetL2BlockState) -> RpcResult<QEDL2BlockState>;

    /// Get latest L2 block state
    #[method(name = "get_latest_l2_block_state")]
    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState>;
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
    CMD: QEDReadCommandProcessorSync<F> + Send + Sync + 'static,
> {
    pub cmd_store: CMD,
    pub ctx: Arc<RealmEdgeContext<SR, DQ, PS>>,
}

#[async_trait]
impl<SR, DQ, PS, CMD> RealmEdgeRpcServer for RealmEdgeRpcImpl<SR, DQ, PS, CMD>
where
    SR: QEDRealmStoreReaderAsync<F> + Send + Sync + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync + 'static,
    PS: QProofStoreAsyncImm
        + QProofStoreReaderAsync
        + QProofStoreWriterAsyncImm
        + Send
        + Sync
        + 'static,
    CMD: QEDReadCommandProcessorSync<F> + Send + Sync + 'static,
{
    /// Implementation of check user ID in realm RPC interface
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool> {
        Ok(self.ctx.includes_user_id(user_id))
    }

    /// Implementation of submit user End Cap proof RPC interface
    async fn submit_user_end_cap(&self, req: QSubmitEndCapRPCRequest<F>) -> RpcResult<bool> {
        self.ctx
            .handle_recv_end_cap_from_user(req.user_ec_input, &req.proof)
            .await
            .map(|_| true)
            .map_err(|e| to_rpc_error::<bool, _>("Failed to process end cap", e).unwrap_err())
    }

    async fn get_batch(
        &self,
        input: QEDReadCommandBatchInput,
    ) -> RpcResult<QEDReadCommandBatchOutput<F>> {
        Ok(self
            .cmd_store
            .resolve_batch(&input)
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_hash(&self, input: QSRHashCmd) -> RpcResult<QHashOut<F>> {
        Ok(self
            .cmd_store
            .resolve_get_hash(&input)
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_merkle_proof(
        &self,
        input: QSRMerkleCmd,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .cmd_store
            .resolve_get_merkle_proof(&input)
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_leaf(&self, input: QSRCmdGetUserLeafData) -> RpcResult<QEDUserLeaf<F>> {
        Ok(self
            .cmd_store
            .resolve_get_user_leaf(&input)
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_contract_leaf(
        &self,
        _input: QSRCmdGetContractLeafData,
    ) -> RpcResult<QEDContractLeaf<F>> {
        RpcError::Anyhow(anyhow::anyhow!("Not implemented")).into()
    }

    async fn get_contract_code(
        &self,
        _input: QSRCmdGetContractCodeDefinition,
    ) -> RpcResult<ContractCodeDefinition> {
        RpcError::Anyhow(anyhow::anyhow!("Not implemented")).into()
    }

    async fn get_checkpoint_leaf(
        &self,
        input: QSRCmdGetCheckpointLeafData,
    ) -> RpcResult<QEDCheckpointLeaf<F>> {
        Ok(self
            .cmd_store
            .resolve_get_checkpoint_leaf(&input)
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_l2_block_state(&self, input: QSRCmdGetL2BlockState) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .cmd_store
            .resolve_get_l2_block_state(&input)
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .cmd_store
            .resolve_get_latest_l2_block_state()
            .map_err(RpcError::Anyhow)?)
    }
}

/// Start Realm Edge node RPC server
pub async fn start_realm_edge_rpc_server<
    CMD: QEDReadCommandProcessorSync<F> + Send + Sync + 'static,
    SR: QEDRealmStoreReaderAsync<F> + Send + Sync + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync + 'static,
    PS: QProofStoreAsyncImm
        + QProofStoreReaderAsync
        + QProofStoreWriterAsyncImm
        + Send
        + Sync
        + 'static,
>(
    cmd_store: CMD,
    ctx: Arc<RealmEdgeContext<SR, DQ, PS>>,
    listen_addr: &str,
) -> Result<jsonrpsee::server::ServerHandle> {
    let server = ServerBuilder::default().build(listen_addr).await?;

    let rpc = RealmEdgeRpcImpl { cmd_store, ctx };

    let handle = server.start(rpc.into_rpc());
    info!("Realm Edge RPC server started on {}", listen_addr);
    Ok(handle)
}
