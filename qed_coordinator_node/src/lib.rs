mod args;
mod edge;
mod processor;
pub use args::*;
pub use edge::*;
pub use processor::*;

pub const COORDINATOR_WORKER_QUEUE_SUFFIX: &str = "wq1";
pub const COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX: &str = "nq1";
pub const COORDINATOR_WORKER_SUFFIX: &str = "CW";
