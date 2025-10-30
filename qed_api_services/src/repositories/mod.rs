// Module declarations
pub mod aggregation;
pub mod metrics;
pub mod stats;
pub mod users;
pub mod workers;
pub mod job_status;
pub mod checkpoint_state;
pub mod worker_event_processor;
pub mod rewards;
pub mod contracts;

// Re-export all repository structs for backward compatibility
pub use aggregation::{
    UserEventAggregationRepository, WorkerEventAggregationRepository, WorkerRewardsAggregationRepository
};
pub use metrics::{TpsRepository, WorkerLeaderboardRepository};
pub use stats::{RealmStatsRepository, WorkerStatsRepository};
pub use users::{UserEventRepository, UserRepository};
pub use workers::WorkerEventRepository;
pub use job_status::*;
pub use contracts::*;