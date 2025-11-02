use psy_core::job::id::QProvingJobDataID;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub job_id: QProvingJobDataID,
    pub location: JobLocation,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub enum JobLocation {
    Realm(u64),
    Coordinator,
}
