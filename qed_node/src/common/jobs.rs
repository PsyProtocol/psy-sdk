use std::collections::HashMap;

use qed_core::job::{
    id::{JobDataIdGraph, QProvingJobDataID},
    traits::JobDataIdGraphReader,
};
use tokio::sync::mpsc;
use tracing::{error, warn};

pub async fn run_jobs_listener<T: JobDataIdGraphReader + Send + Sync + 'static>(
    job_graph_reader: T,
) -> mpsc::Receiver<(u64, JobDataIdGraph)> {
    let (tx, rx) = mpsc::channel(100);
    tokio::spawn(async move {
        loop {
            if let Ok((checkpoint_id, job_graph)) = job_graph_reader.wait_for_next_job_graph().await
            {
                if let Err(e) = tx.send((checkpoint_id, job_graph)).await {
                    error!("Job graph listener channel closed: {:?}", e);
                    break;
                }
            } else {
                warn!("Failed to read job graph");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });
    rx
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Processing,
    Done,
}

pub struct JobsGraphManager {
    pub job_graph: Option<JobDataIdGraph>,
    pub ready_jobs: HashMap<QProvingJobDataID, JobStatus>,
}

impl JobsGraphManager {
    pub fn new() -> Self {
        Self {
            job_graph: None,
            ready_jobs: HashMap::new(),
        }
    }

    pub fn set_job_graph(&mut self, job_graph: JobDataIdGraph) {
        self.job_graph = Some(job_graph);
    }

    fn take_read_job(&mut self) {
        let ready_jobs = self.job_graph.as_mut().map(|graph| graph.take_ready_jobs());
        if let Some(ready_jobs) = ready_jobs.as_ref() {
            for &job_id in ready_jobs.iter() {
                self.ready_jobs.insert(job_id, JobStatus::Pending);
            }
        }
    }

    pub fn get_next_pending_job_to_process(&mut self) -> Option<QProvingJobDataID> {
        if self.ready_jobs.is_empty() {
            self.take_read_job();
        }
        let pending_job = self
            .ready_jobs
            .iter()
            .find(|(_, &status)| status == JobStatus::Pending);
        if let Some((&job_id, _)) = pending_job {
            self.mark_job_status(job_id, JobStatus::Processing);
            Some(job_id)
        } else {
            if let Some(job_graph) = self.job_graph.as_ref() {
                if !job_graph.is_empty() {
                    warn!("Job graph is not empty, but no pending job found");
                }
            }
            None
        }
    }

    pub fn mark_job_status(&mut self, job_id: QProvingJobDataID, new_status: JobStatus) {
        if let Some(status) = self.ready_jobs.get_mut(&job_id) {
            *status = new_status;
        } else {
            warn!("Job not found in ready_jobs: {:?}", job_id);
        }
    }
}
