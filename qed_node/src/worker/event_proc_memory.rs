
use std::{collections::VecDeque, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use qed_core::job::{id::{QProvingJobDataID, QWorkerJobBenchmark}, worker_queue::{WorkerEventReceiverSync, WorkerEventReceiverSyncImm, WorkerEventTransmitterSync, WorkerEventTransmitterSyncImm}};


pub struct QEDEventProcessorMemory {
    pub job_queue: VecDeque<QProvingJobDataID>,
    pub benchmarks_enabled: bool,
    pub benchmarks: Vec<QWorkerJobBenchmark>,
    pub core_job_completed: bool,
}
impl QEDEventProcessorMemory {
    pub fn new() -> Self {
        Self::new_with_config(false)
    }
    pub fn new_with_config(benchmarks_enabled: bool) -> Self {
        Self {
            job_queue: VecDeque::new(),
            benchmarks_enabled,
            benchmarks: Vec::new(),
            core_job_completed: true,
        }
    }
}
impl WorkerEventReceiverSync for QEDEventProcessorMemory {
    fn wait_for_next_job_mut(&mut self) -> anyhow::Result<QProvingJobDataID> {
        if self.job_queue.is_empty() {
            Err(anyhow::format_err!("No jobs in queue, note that QEDEventProcessorMemory::wait_for_next_job does not block the thread like other implementations of WorkerEventReceiverSync do."))
        } else {
            Ok(self.job_queue.pop_front().unwrap())
        }
    }

    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        self.core_job_completed = false;
        self.job_queue.extend(jobs.into_iter());
        Ok(())
    }

    fn notify_core_goal_completed_mut(&mut self, _job: QProvingJobDataID) -> anyhow::Result<()> {
        self.core_job_completed = true;
        Ok(())
    }

    fn record_job_bench_mut(&mut self, job: QProvingJobDataID, duration: u64) -> anyhow::Result<()> {
        if self.benchmarks_enabled {
            self.benchmarks.push(QWorkerJobBenchmark {
                job_id: job.to_fixed_bytes(),
                duration,
            });
        }
        Ok(())
    }
}

impl WorkerEventTransmitterSync for QEDEventProcessorMemory {
    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        self.job_queue.extend(jobs.into_iter());
        Ok(())
    }

    fn wait_for_block_proving_jobs_mut(&mut self, _checkpoint_id: u64) -> anyhow::Result<bool> {
        if !self.core_job_completed {
            anyhow::bail!("core job not yet completed!");
        }
        //tracing::info!("QEDEventProcessorMemory::wait_for_block_proving_jobs is a no-op since its for local (sync) testing only.");
        Ok(false)
    }
}



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
    pub fn dup(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn write(&self) -> anyhow::Result<RwLockWriteGuard<P>> {
        self.inner
            .try_write()
            .map_err(|err| anyhow::anyhow!("Error writing to immutable store: {:?}", err))
    }
    pub fn read(&self) -> anyhow::Result<RwLockReadGuard<P>> {
        self.inner
            .try_read()
            .map_err(|err| anyhow::anyhow!("Error reading from immutable store: {:?}", err))
    }
}


impl<PM: WorkerEventReceiverSync> WorkerEventReceiverSyncImm for QEDArcImmutableEventProcessorWrapper<PM> {
    fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID> {
        self.write()?.wait_for_next_job_mut()
    }

    fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        self.write()?.enqueue_jobs_mut(jobs)
    }

    fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        self.write()?.notify_core_goal_completed_mut(job)
    }

    fn record_job_bench_imm(&self, job: QProvingJobDataID, duration: u64) -> anyhow::Result<()> {
        self.write()?.record_job_bench_mut(job, duration)
    }
}

impl<PM: WorkerEventTransmitterSync> WorkerEventTransmitterSyncImm for QEDArcImmutableEventProcessorWrapper<PM> {
    fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        self.write()?.enqueue_jobs_mut(jobs)
    }

    fn wait_for_block_proving_jobs_imm(&self, checkpoint_id: u64) -> anyhow::Result<bool> {
        self.write()?.wait_for_block_proving_jobs_mut(checkpoint_id)
    }
}
