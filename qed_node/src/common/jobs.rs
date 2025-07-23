use std::{collections::HashMap, sync::Arc};

use qed_core::job::{
    id::{JobDataIdGraph, QProvingJobDataID},
    traits::JobDataIdGraphReader,
};
use tokio::sync::Mutex;
use tracing::{error, warn};

pub async fn run_jobs_listener<T: JobDataIdGraphReader + Send + Sync + 'static>(
    job_manager: Arc<Mutex<JobsGraphManager>>,
    job_graph_reader: Arc<T>,
) {
    tokio::spawn(async move {
        loop {
            if let Ok((checkpoint_id, job_graph)) = job_graph_reader.wait_for_next_job_graph().await
            {
                let mut job_manager = job_manager.lock().await;
                if job_manager.is_job_graph_empty() {
                    job_manager.set_job_graph(job_graph, checkpoint_id);
                } else {
                    error!(
                        "🔍 Job graph is not empty, skipping, checkpoint_id = {}, job_graph = {:?}",
                        checkpoint_id, job_graph
                    );
                }
            } else {
                warn!("Failed to read job graph");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });
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
    pub checkpoint_id: u64,
}

impl JobsGraphManager {
    pub fn new() -> Self {
        Self {
            job_graph: None,
            ready_jobs: HashMap::new(),
            checkpoint_id: 0,
        }
    }

    pub fn set_job_graph(&mut self, job_graph: JobDataIdGraph, checkpoint_id: u64) {
        self.job_graph = Some(job_graph);
        self.checkpoint_id = checkpoint_id;
    }

    pub fn is_job_graph_empty(&self) -> bool {
        self.job_graph
            .as_ref()
            .map(|graph| graph.is_empty())
            .unwrap_or(true)
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

    pub fn mark_job_done(&mut self, job_id: QProvingJobDataID) {
        self.mark_job_status(job_id, JobStatus::Done);
    }

    pub fn mark_job_status(&mut self, job_id: QProvingJobDataID, new_status: JobStatus) {
        if let Some(status) = self.ready_jobs.get_mut(&job_id) {
            *status = new_status;
        } else {
            warn!("Job not found in ready_jobs: {:?}", job_id);
        }
    }
}
