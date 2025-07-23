use std::collections::HashMap;

use qed_core::job::id::{JobDataIdGraph, QProvingJobDataID};
use tracing::warn;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JobIdStatus {
    Pending,
    Processing,
    Done,
}

pub struct JobScheduler {
    pub job_graph: Option<JobDataIdGraph>,
    pub ready_jobs: HashMap<QProvingJobDataID, JobIdStatus>,
}

impl JobScheduler {
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
                self.ready_jobs.insert(job_id, JobIdStatus::Pending);
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
            .find(|(_, &status)| status == JobIdStatus::Pending);
        if let Some((&job_id, _)) = pending_job {
            self.mark_job_status(job_id, JobIdStatus::Processing);
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

    pub fn mark_job_status(&mut self, job_id: QProvingJobDataID, new_status: JobIdStatus) {
        if let Some(status) = self.ready_jobs.get_mut(&job_id) {
            *status = new_status;
        } else {
            warn!("Job not found in ready_jobs: {:?}", job_id);
        }
    }
}
// processor写chain state
//
