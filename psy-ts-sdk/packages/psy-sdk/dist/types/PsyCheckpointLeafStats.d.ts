import type { PMRewardCommitment } from "./PMRewardCommitment";
import type { PMJobsCompletedStats } from "./PMJobsCompletedStats";
import type { QHashOut } from "./QHashOut";
export type PsyCheckpointLeafStats = {
    fees_collected: bigint;
    user_ops_processed: bigint;
    total_transactions: bigint;
    slots_modified: bigint;
    pm_jobs_completed: PMJobsCompletedStats;
    block_time: bigint;
    random_seed: QHashOut;
    pm_rewards_commitment: PMRewardCommitment;
    da_challenges_claimed: [bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint];
};
//# sourceMappingURL=PsyCheckpointLeafStats.d.ts.map