use plonky2::{field::goldilocks_field::GoldilocksField, hash::{hash_types::RichField, poseidon::PoseidonHash}, plonk::config::Hasher};
use qed_core::traits::to_qfelts::{QFeltSized, ToQFelts};
use serde::{Deserialize, Serialize};
use ts_rs::TS;


// TODO: Make a constant size commitment scheme for proof miners
// for now, we can just use a partial merkle tree for testing,  
// but in the future we want miners to be able to prove that they participated
// by only knowing the final block commitment + the element which proves their participation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default,TS)]
#[ts(export, concrete(F = GoldilocksField))]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct PMRewardCommitment<F: RichField> {
    pub commitment: [F; 4],
}



impl<F: RichField> PMRewardCommitment<F> {
    // TODO: Implement a proper commitment scheme
    pub fn combine_with(&self, other: &Self) -> Self {
        let commitment = PoseidonHash::hash_no_pad(&[
            self.commitment[0],
            self.commitment[1],
            self.commitment[2],
            self.commitment[3],
            other.commitment[0],
            other.commitment[1],
            other.commitment[2],
            other.commitment[3],
        ]).elements;
        PMRewardCommitment {
            commitment
        }
    }
}
impl<F: RichField> QFeltSized for PMRewardCommitment<F> {
    fn q_felt_size() -> usize {
        4
    }
}
impl<F: RichField> ToQFelts<F> for PMRewardCommitment<F> {
    fn to_qfelts(&self) -> Vec<F> {
        self.commitment.to_vec()
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() != 4 {
            panic!("Invalid number of elements for PMRewardCommitment");
        }
        PMRewardCommitment {
            commitment: [felts[0], felts[1], felts[2], felts[3]]
        }
    }
}