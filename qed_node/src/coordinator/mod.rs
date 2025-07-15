pub mod state_helper;
pub mod edge;
pub mod state;
pub mod demo;
pub mod args;
pub mod processor;

pub use args::*;
pub use processor::*;

pub const COORDINATOR_WORKER_QUEUE_SUFFIX: &str = "wq1";
pub const COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX: &str = "nq1";
pub const COORDINATOR_WORKER_SUFFIX: &str = "CW";