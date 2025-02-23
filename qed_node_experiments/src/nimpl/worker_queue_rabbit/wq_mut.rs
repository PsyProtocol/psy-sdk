use std::time::Duration;

use qed_core::job::{id::{QProvingJobDataID, QWorkerJobBenchmark}, worker_queue::{ProvingDispatcher, ProvingWorkerListener, WorkerEventReceiverAsync, WorkerEventReceiverSync, WorkerEventTransmitterAsync}};
use qed_node_common::worker::event_proc_memory::QEDArcImmutableEventProcessorWrapper;
use temporal_client::tonic::async_trait;

use crate::nimpl::worker_queue_redis::redis_queue::{QueueNotification, RedisQueue, Q_NOTIFICATIONS};

use super::rabbit_mq_queue::RabbitMQQueue;


pub struct QEDRabbitMQEventProcessor {
    pub job_queue: RabbitMQQueue,
    pub redis_job_queue: RedisQueue,
    pub benckmarks_enabled: bool,
    pub benchmarks: Vec<QWorkerJobBenchmark>,
}
impl QEDRabbitMQEventProcessor {
    pub fn new(dispatcher: RabbitMQQueue, redis_job_queue: RedisQueue) -> Self {
        Self::new_with_config(dispatcher, redis_job_queue, false)
    }
    pub fn new_with_config(dispatcher: RabbitMQQueue, redis_job_queue: RedisQueue, benckmarks_enabled: bool) -> Self {
        Self {
            job_queue: dispatcher,
            redis_job_queue,
            benckmarks_enabled,
            benchmarks: Vec::new(),
        }
    }
    pub fn to_imm(self) -> QEDArcImmutableEventProcessorWrapper<Self> {
        QEDArcImmutableEventProcessorWrapper::new(self)
    }
}
#[async_trait]
impl WorkerEventReceiverAsync for QEDRabbitMQEventProcessor {
    async fn wait_for_next_job_mut(&mut self) -> anyhow::Result<QProvingJobDataID> {
        self.job_queue.get_next_task().await
    }

    async fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        //println!("got new jobs: {:?}",jobs);
       self.job_queue.push_tasks(jobs).await
    }

    async fn notify_core_goal_completed_mut(&mut self, _job: QProvingJobDataID) -> anyhow::Result<()> {
        self.redis_job_queue.dispatch(Q_NOTIFICATIONS, QueueNotification::CoreJobCompleted)?;
        Ok(())
    }
    
    fn record_job_bench_mut(&mut self, job: QProvingJobDataID, duration: u64) -> anyhow::Result<()> {
        if self.benckmarks_enabled {
            self.benchmarks.push(QWorkerJobBenchmark {
                job_id: job.to_fixed_bytes(),
                duration,
            });
        }
        Ok(())
    }
}

#[async_trait]
impl WorkerEventTransmitterAsync for QEDRabbitMQEventProcessor {
    async fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        self.job_queue.push_tasks(jobs).await?;
        Ok(())
    }

    async fn wait_for_block_proving_jobs_mut(&mut self, _checkpoint_id: u64) -> anyhow::Result<bool> {
        loop {
            match self
                .redis_job_queue
                .pop_one(Q_NOTIFICATIONS)?
                .map(|v| serde_json::from_slice::<QueueNotification>(&v))
            {
                Some(Ok(QueueNotification::CoreJobCompleted)) => return Ok::<_, anyhow::Error>(true),
                Some(Err(_)) | None => {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }
    }
}


