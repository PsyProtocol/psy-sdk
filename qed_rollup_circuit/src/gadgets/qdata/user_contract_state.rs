use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOutTarget, RichField},
    iop::{target::Target, witness::Witness},
    plonk::circuit_builder::CircuitBuilder,
};
use qed_common_circuit::traits::CreatableTarget;
use qed_data::qdata::user_contract_state::UserContractState;

use super::user::QEDUserLeafGadget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserContractStateGadget {
    pub checkpoint_tree_root: HashOutTarget,
    pub user_leaf: QEDUserLeafGadget,
}

impl UserContractStateGadget {
    pub fn add_virtual_to<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let checkpoint_tree_root = builder.add_virtual_hash();
        let user_leaf = QEDUserLeafGadget::create_virtual(builder);
        Self {
            // checkpoint_id: builder.add_virtual_target(),
            checkpoint_tree_root,
            // checkpoint_leaf_hash: builder.add_virtual_hash(),
            user_leaf,
        }
    }

    pub fn set_witness<F: RichField>(
        &self,
        witness: &mut impl Witness<F>,
        target: &UserContractState<F>,
    ) -> anyhow::Result<()> {
        witness.set_hash_target(self.checkpoint_tree_root, target.checkpoint_tree_root.0)?;
        self.user_leaf.set_witness(witness, &target.user_leaf)
    }

    pub fn to_targets(&self) -> Vec<Target> {
        vec![
            self.checkpoint_tree_root.elements[0],
            self.checkpoint_tree_root.elements[1],
            self.checkpoint_tree_root.elements[2],
            self.checkpoint_tree_root.elements[3],
            self.user_leaf.public_key.elements[0],
            self.user_leaf.public_key.elements[1],
            self.user_leaf.public_key.elements[2],
            self.user_leaf.public_key.elements[3],
            self.user_leaf.user_state_tree_root.elements[0],
            self.user_leaf.user_state_tree_root.elements[1],
            self.user_leaf.user_state_tree_root.elements[2],
            self.user_leaf.user_state_tree_root.elements[3],
            self.user_leaf.balance,
            self.user_leaf.nonce,
            self.user_leaf.last_checkpoint_id,
            self.user_leaf.event_index,
            self.user_leaf.user_id,
        ]
    }
}
