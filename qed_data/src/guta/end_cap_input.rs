use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::{hasher::FieldQHasher, qhashable::QFieldHashable};
use serde::{Deserialize, Serialize};

use crate::qstore::uct_merkle_nodes::{CSTUserUpdate, CSTUserUpdateStore};

use super::api::{QEDContractStateUpdateHistory, SubmitUserEndCapNonProofCoreInput};


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitUserEndCapNonProofInput<F: RichField> {
    pub core: SubmitUserEndCapNonProofCoreInput<F>,
    pub contract_state_updates: Vec<QEDContractStateUpdateHistory<F>>,
}
impl<F: RichField> SubmitUserEndCapNonProofInput<F> {
    pub fn ensure_simple_self_consistent<H: FieldQHasher<F>>(&self, proof_public_inputs_hash: QHashOut<F>) -> anyhow::Result<()> {
        if self.core.checkpoint_id != self.core.new_user_leaf.last_checkpoint_id {
            anyhow::bail!("invalid checkpoint id");
        }
        if self.core.new_user_leaf.user_id != self.core.state_transition.user_id {
            anyhow::bail!("inconsistent user id");
        }

        let expected_proof_public_inputs_hash = self.core.get_proof_public_inputs_hash::<H>();
        if proof_public_inputs_hash != expected_proof_public_inputs_hash {
            anyhow::bail!("invalid public inputs/state transition");
        }


        let computed_leaf_hash = self.core.new_user_leaf.qfhash::<H>();
        if computed_leaf_hash != self.core.state_transition.end_user_leaf_hash {
            anyhow::bail!("invalid new_user_leaf");
        }
        if self.contract_state_updates.len() == 0 {
            anyhow::bail!("contract_state_updates cannot be empty");
        }

        if self.contract_state_updates.last().as_ref().unwrap().user_contract_tree_update_proof.new_root != self.core.new_user_leaf.user_state_tree_root {
            anyhow::bail!("user_state_tree_root does not match the last new root");
        }

        for csu in self.contract_state_updates.iter() {
            csu.ensure_basic_consistency()?;
        }

        

        Ok(())
    }
    pub fn verify_and_generate_cst_updates<H: FieldQHasher<F>>(&self, checkpoint_id: u64, old_user_state_tree_root: QHashOut<F>) -> anyhow::Result<CSTUserUpdate<QHashOut<F>>> {

        if self.contract_state_updates.len() == 0 {
            anyhow::bail!("contract_state_updates cannot be empty");
        }


        if self.contract_state_updates[0].user_contract_tree_update_proof.old_root != old_user_state_tree_root {
            anyhow::bail!("old_user_state_tree_root does not match the first old root");
        }
        let mut injestor = CSTUserUpdateStore::<QHashOut<F>>::new();
        
        for csu in self.contract_state_updates.iter() {
            csu.verify_generate_cst_delta::<H>(&mut injestor)?;
        }

        let upd = injestor.into_updates(checkpoint_id, self.core.state_transition.user_id.to_canonical_u64());



        Ok(upd)
    }

}

impl<F: RichField> KVQSerializable for SubmitUserEndCapNonProofInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
