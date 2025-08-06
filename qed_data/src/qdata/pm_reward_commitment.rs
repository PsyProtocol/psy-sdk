use plonky2::{field::goldilocks_field::GoldilocksField, hash::{hash_types::RichField, poseidon::PoseidonHash}, plonk::config::Hasher};
use qed_core::{data::qhashout::QHashOut, traits::to_qfelts::{QFeltSized, ToQFelts}};
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
    pub register_users_root: QHashOut<F>,
    pub gutas_root: QHashOut<F>,
    pub deploy_contracts_root: QHashOut<F>,
}



impl<F: RichField> PMRewardCommitment<F> {
    // Combine two PMRewardCommitments by hashing the roots together
    pub fn combine_with(&self, other: &Self) -> Self {
        let register_users_root = QHashOut(PoseidonHash::two_to_one(
            self.register_users_root.0,
            other.register_users_root.0,
        ));
        let gutas_root = QHashOut(PoseidonHash::two_to_one(
            self.gutas_root.0,
            other.gutas_root.0,
        ));
        let deploy_contracts_root = QHashOut(PoseidonHash::two_to_one(
            self.deploy_contracts_root.0,
            other.deploy_contracts_root.0,
        ));
        PMRewardCommitment {
            register_users_root,
            gutas_root,
            deploy_contracts_root,
        }
    }
    
    // Get the overall commitment hash
    pub fn get_commitment_hash(&self) -> QHashOut<F> {
        let temp = PoseidonHash::two_to_one(
            self.register_users_root.0,
            self.gutas_root.0,
        );
        QHashOut(PoseidonHash::two_to_one(
            temp,
            self.deploy_contracts_root.0,
        ))
    }
}
impl<F: RichField> QFeltSized for PMRewardCommitment<F> {
    fn q_felt_size() -> usize {
        12 // 3 QHashOut, each with 4 fields
    }
}
impl<F: RichField> ToQFelts<F> for PMRewardCommitment<F> {
    fn to_qfelts(&self) -> Vec<F> {
        let mut result = Vec::with_capacity(12);
        result.extend_from_slice(&self.register_users_root.0.elements);
        result.extend_from_slice(&self.gutas_root.0.elements);
        result.extend_from_slice(&self.deploy_contracts_root.0.elements);
        result
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() != 12 {
            panic!("Invalid number of elements for PMRewardCommitment, expected 12 got {}", felts.len());
        }
        PMRewardCommitment {
            register_users_root: QHashOut(plonky2::hash::hash_types::HashOut {
                elements: [felts[0], felts[1], felts[2], felts[3]],
            }),
            gutas_root: QHashOut(plonky2::hash::hash_types::HashOut {
                elements: [felts[4], felts[5], felts[6], felts[7]],
            }),
            deploy_contracts_root: QHashOut(plonky2::hash::hash_types::HashOut {
                elements: [felts[8], felts[9], felts[10], felts[11]],
            }),
        }
    }
}