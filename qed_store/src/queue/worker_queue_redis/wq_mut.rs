use std::time::Duration;

use qed_core::job::{id::{QProvingJobDataID, QWorkerJobBenchmark}, worker_queue::{ProvingDispatcher, ProvingWorkerListener, WorkerEventReceiverSync, WorkerEventTransmitterSync}};
// TODO: QEDArcImmutableEventProcessorWrapper should be moved to qed_core
// For now, we'll define it here to avoid circular dependency
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct QEDArcImmutableEventProcessorWrapper<P> {
    pub inner: Arc<RwLock<P>>,
}

impl<P> QEDArcImmutableEventProcessorWrapper<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}

use super::redis_queue::{QueueNotification, RedisQueue, Q_JOB, Q_NOTIFICATIONS};


#[derive(Clone)]
pub struct QEDRedisEventProcessor {
    pub job_queue: RedisQueue,
    pub benckmarks_enabled: bool,
    pub benchmarks: Vec<QWorkerJobBenchmark>,
}
impl QEDRedisEventProcessor {
    pub fn new(dispatcher: RedisQueue) -> Self {
        Self::new_with_config(dispatcher, false)
    }
    pub fn new_with_config(dispatcher: RedisQueue, benckmarks_enabled: bool) -> Self {
        Self {
            job_queue: dispatcher,
            benckmarks_enabled,
            benchmarks: Vec::new(),
        }
    }
    pub fn to_imm(self) -> QEDArcImmutableEventProcessorWrapper<Self> {
        QEDArcImmutableEventProcessorWrapper::new(self)
    }
}
impl WorkerEventReceiverSync for QEDRedisEventProcessor {
    fn wait_for_next_job_mut(&mut self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            let job = self.job_queue.pop_one(Q_JOB)?;
            if job.is_some() {
                return Ok(serde_json::from_slice(&job.unwrap())?)
            }else{
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
        }
    }

    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        for job in jobs {
            self.job_queue.dispatch(Q_JOB, job.clone())?;
        }
        Ok(())
    }

    fn notify_core_goal_completed_mut(&mut self, _job: QProvingJobDataID) -> anyhow::Result<()> {
        self.job_queue.dispatch(Q_NOTIFICATIONS, QueueNotification::CoreJobCompleted)?;
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

impl WorkerEventTransmitterSync for QEDRedisEventProcessor {
    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        for job in jobs {
            self.job_queue.dispatch(Q_JOB, job.clone())?;
        }
        Ok(())
    }

    fn wait_for_block_proving_jobs_mut(&mut self, _checkpoint_id: u64) -> anyhow::Result<bool> {
        loop {
            match self
                .job_queue
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


