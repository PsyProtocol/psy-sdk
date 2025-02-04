use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::{target::Target, witness::Witness}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_common_circuit::traits::{AlgebraicHashableTarget, CreatableTarget, FromTargets, ToTargets, WitnessValueFor};
use qed_core::config::network_constants::DA_CHALLENGE_WINDOW;
use qed_data::qdata::checkpoint::QEDCheckpointLeafStats;

use super::pm_reward_commitment::{PMRewardCommitmentGadget, PM_REWARD_COMMITMENT_TARGET_SIZE};

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct QEDCheckpointLeafStatsGadget {
    pub fees_collected: Target,

    pub user_ops_processed: Target,
    pub total_transactions: Target,

    pub slots_modified: Target,
    pub pm_jobs_completed: Target,

    pub block_time: Target,
    
    pub random_seed: HashOutTarget,
    pub pm_rewards_commitment: PMRewardCommitmentGadget,

    // data availability miner proofs successfully completed for the previous 16 blocks
    pub da_challenges_claimed: [Target; DA_CHALLENGE_WINDOW],
}

impl QEDCheckpointLeafStatsGadget {
    pub fn set_witness<F: RichField>(&self, witness: &mut impl Witness<F>, target: &QEDCheckpointLeafStats<F>) {
        witness.set_target(self.fees_collected, target.fees_collected);

        witness.set_target(self.user_ops_processed, target.user_ops_processed);
        witness.set_target(self.total_transactions, target.total_transactions);
        
        witness.set_target(self.slots_modified, target.slots_modified);
        witness.set_target(self.pm_jobs_completed, target.pm_jobs_completed);
        
        witness.set_target(self.block_time, target.block_time);

        witness.set_hash_target(self.random_seed, target.random_seed.0);
        self.pm_rewards_commitment.set_witness(witness, &target.pm_rewards_commitment);

        witness.set_target_arr(&self.da_challenges_claimed, &target.da_challenges_claimed);
    }
    pub fn to_hash<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        builder.hash_n_to_hash_no_pad::<H>(self.to_targets())
    }
}
impl AlgebraicHashableTarget for QEDCheckpointLeafStatsGadget {
    fn to_hash_target<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        self.to_hash::<H, F, D>(builder)
    }
}
impl CreatableTarget for QEDCheckpointLeafStatsGadget {
    fn create_virtual<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let fees_collected = builder.add_virtual_target();
        let user_ops_processed = builder.add_virtual_target();
        let total_transactions = builder.add_virtual_target();
        let slots_modified = builder.add_virtual_target();
        let pm_jobs_completed = builder.add_virtual_target();
        let block_time = builder.add_virtual_target();



        let random_seed = builder.add_virtual_hash();
        let pm_rewards_commitment = PMRewardCommitmentGadget::create_virtual(builder);
        let da_challenges_claimed = builder.add_virtual_target_arr::<DA_CHALLENGE_WINDOW>();

        Self {
            fees_collected,
            user_ops_processed,
            total_transactions,
            slots_modified,
            pm_jobs_completed,
            block_time,
            random_seed,
            pm_rewards_commitment,
            da_challenges_claimed,
        }
        
    }
}
impl ToTargets for QEDCheckpointLeafStatsGadget {
    fn to_targets(&self) -> Vec<Target> {
        let mut result = vec![
            self.fees_collected,
            self.user_ops_processed,
            self.total_transactions,

            self.slots_modified,
            self.pm_jobs_completed,
            self.block_time,
            self.random_seed.elements[0],
            self.random_seed.elements[1],
            self.random_seed.elements[2],
            self.random_seed.elements[3],
        ];
        result.extend_from_slice(&self.pm_rewards_commitment.to_targets());
        result.extend_from_slice(&self.da_challenges_claimed);
        result
    }
}
impl FromTargets for QEDCheckpointLeafStatsGadget {
    fn from_targets(targets: &[Target]) -> Self {
        if targets.len() != 10 + PM_REWARD_COMMITMENT_TARGET_SIZE + DA_CHALLENGE_WINDOW {
            panic!("Invalid number of elements for QEDCheckpointLeafStats");
        }
        QEDCheckpointLeafStatsGadget {
            fees_collected: targets[0],
            user_ops_processed: targets[1],
            total_transactions: targets[2],
            slots_modified: targets[3],
            pm_jobs_completed: targets[4],
            block_time: targets[5],
            random_seed: HashOutTarget {
                elements: [
                    targets[6],
                    targets[7],
                    targets[8],
                    targets[9],
                ]
            },
            pm_rewards_commitment: PMRewardCommitmentGadget::from_targets(&targets[10..(10+PM_REWARD_COMMITMENT_TARGET_SIZE)]),
            da_challenges_claimed: targets[(10+PM_REWARD_COMMITMENT_TARGET_SIZE)..].try_into().unwrap(),
            
        }
    }
}


impl<F: RichField> WitnessValueFor<QEDCheckpointLeafStatsGadget, F, true> for QEDCheckpointLeafStats<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDCheckpointLeafStatsGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<QEDCheckpointLeafStatsGadget, F, false> for QEDCheckpointLeafStats<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDCheckpointLeafStatsGadget) {
        target.set_witness(witness, self);
    }
}



