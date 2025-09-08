// Module declarations
pub mod aggregation;
pub mod metrics;
pub mod rewards;
pub mod stats;
pub mod users;
pub mod workers;

// Re-export all repository structs for backward compatibility
pub use aggregation::{
    UserEventAggregationRepository, WorkerEventAggregationRepository, WorkerRewardsAggregationRepository
};
pub use metrics::{TpsRepository, WorkerLeaderboardRepository};
pub use rewards::{WorkerEventRewardRepository, WorkerRewardsRepository};
pub use stats::{RealmStatsRepository, WorkerStatsRepository};
pub use users::{UserEventRepository, UserRepository};
pub use workers::WorkerEventRepository;