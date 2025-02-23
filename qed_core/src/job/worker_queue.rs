use std::time::Duration;

use serde::Serialize;

use super::id::QProvingJobDataID;
use async_trait::async_trait;

#[async_trait]
pub trait WorkerEventReceiverAsync {
    async fn wait_for_next_job_mut(&mut self) -> anyhow::Result<QProvingJobDataID>;
    async fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    async fn notify_core_goal_completed_mut(&mut self, job: QProvingJobDataID) -> anyhow::Result<()>;
    fn record_job_bench_mut(&mut self, job: QProvingJobDataID, duration: u64)
        -> anyhow::Result<()>;
}
#[async_trait]
pub trait WorkerEventReceiverAsyncImm {
    async fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID>;
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()>;
}

#[async_trait]
pub trait WorkerEventTransmitterAsync {
    async fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    async fn wait_for_block_proving_jobs_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<bool>;
}


#[async_trait]
pub trait WorkerEventTransmitterAsyncImm {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    async fn wait_for_block_proving_jobs_imm(&self, checkpoint_id: u64) -> anyhow::Result<bool>;
}

pub trait WorkerEventReceiverSync {
    fn wait_for_next_job_mut(&mut self) -> anyhow::Result<QProvingJobDataID>;
    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    fn notify_core_goal_completed_mut(&mut self, job: QProvingJobDataID) -> anyhow::Result<()>;
    fn record_job_bench_mut(&mut self, job: QProvingJobDataID, duration: u64)
        -> anyhow::Result<()>;
}

pub trait WorkerEventTransmitterSync {
    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    fn wait_for_block_proving_jobs_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<bool>;
}

pub trait WorkerEventReceiverSyncImm {
    fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID>;
    fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()>;
    fn record_job_bench_imm(&self, job: QProvingJobDataID, duration: u64) -> anyhow::Result<()>;
}

pub trait WorkerEventTransmitterSyncImm {
    fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    fn wait_for_block_proving_jobs_imm(&self, checkpoint_id: u64) -> anyhow::Result<bool>;
}

pub trait ProvingDispatcher {
    fn dispatch(
        &mut self,
        topic: &'static str,
        value: impl Serialize + Send + 'static,
    ) -> anyhow::Result<()>;
}

pub trait ProvingWorkerListener: ProvingDispatcher {
    fn subscribe(&mut self, topic: &'static str) -> anyhow::Result<()>;
    fn receive_one(
        &mut self,
        topic: &'static str,
        hidden: Option<Duration>,
    ) -> anyhow::Result<Option<(String, Vec<u8>)>>;
    fn pop_one(&mut self, topic: &'static str) -> anyhow::Result<Option<Vec<u8>>>;
    fn receive_all(
        &mut self,
        topic: &'static str,
        hidden: Option<Duration>,
    ) -> anyhow::Result<Vec<(String, Vec<u8>)>>;
    fn pop_all(&mut self, topic: &'static str) -> anyhow::Result<Vec<Vec<u8>>>;
    fn delete_message(&mut self, topic: &'static str, id: String) -> anyhow::Result<bool>;
    fn is_empty(&mut self) -> bool;
}
