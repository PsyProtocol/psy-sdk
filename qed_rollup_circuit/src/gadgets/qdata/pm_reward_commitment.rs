use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::{target::Target, witness::Witness}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_common_circuit::traits::{AlgebraicHashableTarget, CreatableTarget, FromTargets, ToTargets, WitnessValueFor};
use qed_data::qdata::pm_reward_commitment::PMRewardCommitment;


pub const PM_REWARD_COMMITMENT_TARGET_SIZE: usize = 4;
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct PMRewardCommitmentGadget {
    pub commitment: [Target; 4],
}

impl PMRewardCommitmentGadget {
    pub fn set_witness<F: RichField>(&self, witness: &mut impl Witness<F>, target: &PMRewardCommitment<F>) {

        witness.set_target(self.commitment[0], target.commitment[0]);
        witness.set_target(self.commitment[1], target.commitment[1]);
        witness.set_target(self.commitment[2], target.commitment[2]);
        witness.set_target(self.commitment[3], target.commitment[3]);
    }
    pub fn to_hash<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        builder.hash_n_to_hash_no_pad::<H>(self.to_targets())
    }
}
impl AlgebraicHashableTarget for PMRewardCommitmentGadget {
    fn to_hash_target<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        self.to_hash::<H, F, D>(builder)
    }
}
impl CreatableTarget for PMRewardCommitmentGadget {
    fn create_virtual<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let commitment = builder.add_virtual_target_arr::<4>();
        Self {
            commitment,
        }
        
    }
}
impl ToTargets for PMRewardCommitmentGadget {
    fn to_targets(&self) -> Vec<Target> {
        vec![
            self.commitment[0],
            self.commitment[1],
            self.commitment[2],
            self.commitment[3],
        ]
    }
}
impl FromTargets for PMRewardCommitmentGadget {
    fn from_targets(targets: &[Target]) -> Self {
        if targets.len() != 4 {
            panic!("tried to create PMRewardCommitmentGadget from an array of {} targets, but expected an array of 4 targets", targets.len());
        }

        let commitment = [
            targets[0],
            targets[1],
            targets[2],
            targets[3],
        ];
        Self {
            commitment,
        }
    }
}


impl<F: RichField> WitnessValueFor<PMRewardCommitmentGadget, F, true> for PMRewardCommitment<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &PMRewardCommitmentGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<PMRewardCommitmentGadget, F, false> for PMRewardCommitment<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &PMRewardCommitmentGadget) {
        target.set_witness(witness, self);
    }
}
