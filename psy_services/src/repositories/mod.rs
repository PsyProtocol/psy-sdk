// Module declarations
pub mod aggregation;
pub mod job_status;
pub mod metrics;
pub mod rewards;
pub mod stats;
pub mod users;
pub mod workers;

// Re-export all repository structs for backward compatibility
pub use aggregation::{UserEventAggregationRepository, WorkerEventAggregationRepository, WorkerRewardsAggregationRepository};
pub use job_status::*;
pub use metrics::{TpsRepository, WorkerLeaderboardRepository};
pub use rewards::{WorkerEventRewardRepository, WorkerRewardsRepository};
pub use stats::{RealmStatsRepository, WorkerStatsRepository};
pub use users::{UserEventRepository, UserRepository};
pub use workers::WorkerEventRepository;
