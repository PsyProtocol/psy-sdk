mod processor;
pub use processor::*;

use jsonrpsee::core::async_trait;
use jsonrpsee::types::ErrorObjectOwned;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use qed_core::job::drain_queue::{
    CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm,
};
use qed_core::job::id::QProvingJobDataID;
use qed_core::job::traits::{
    QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm,
};
use qed_store::config::store_config::QEDFelt;
use qed_store::node::realm::{QEDRealmStoreReaderAsync, QEDRealmStoreWriterAsyncImm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use plonky2::{field::types::Field, plonk::config::PoseidonGoldilocksConfig};
use qed_core::job::history_queue::CheckpointHistoryQueueConsumerAsyncImm;
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_store::node::coordinator::store_traits::{
    QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
};
use tokio::sync::Mutex;

pub type C = PoseidonGoldilocksConfig;
pub const D: usize = 2;

#[rpc(server)]
pub(crate) trait RealmProcessorRpc {
    #[method(name = "build_block")]
    async fn build_block(&self) -> RpcResult<QProvingJobDataID>;
}

pub struct RealmProcessorRpc {
    pub processor: Arc<Mutex<RealmProcessor>>,
}

#[async_trait]
impl RealmProcessorRpcServer for RealmProcessorRpc {
    async fn build_block(&self) -> RpcResult<QProvingJobDataID> {
        let mut guard = self.processor.lock().await;
        let id = guard.build_block().await.map_err(|e| {
            ErrorObjectOwned::owned(
                jsonrpsee::types::error::UNKNOWN_ERROR_CODE,
                e.to_string(),
                None::<String>,
            )
        })?;
        Ok(id)
    }
}
