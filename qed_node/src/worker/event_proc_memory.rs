
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
