use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::{target::Target, witness::Witness}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_common_circuit::{builder::hash::core::CircuitBuilderHashCore, traits::{AlgebraicHashableTarget, CreatableTarget, FromTargets, ToTargets, WitnessValueFor}};
use qed_data::qdata::checkpoint::QEDCheckpointGlobalStateRoots;

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct QEDCheckpointGlobalStateRootsGadget {
    pub contract_tree_root: HashOutTarget,
    pub deposit_tree_root: HashOutTarget,
    pub user_tree_root: HashOutTarget,
    pub withdrawal_tree_root: HashOutTarget,
}

impl QEDCheckpointGlobalStateRootsGadget {
    pub fn set_witness<F: RichField>(&self, witness: &mut impl Witness<F>, target: &QEDCheckpointGlobalStateRoots<F>) -> anyhow::Result<()> {
        witness.set_hash_target(self.contract_tree_root, target.contract_tree_root.0)?;
        witness.set_hash_target(self.deposit_tree_root, target.deposit_tree_root.0)?;
        witness.set_hash_target(self.user_tree_root, target.user_tree_root.0)?;
        witness.set_hash_target(self.withdrawal_tree_root, target.withdrawal_tree_root.0)?;
        Ok(())
    }
    pub fn to_hash<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        let left = builder.hash_two_to_one::<H>(self.contract_tree_root, self.deposit_tree_root);
        let right = builder.hash_two_to_one::<H>(self.user_tree_root, self.withdrawal_tree_root);
        builder.hash_two_to_one::<H>(left, right)
    }
}
impl AlgebraicHashableTarget for QEDCheckpointGlobalStateRootsGadget {
    fn to_hash_target<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        self.to_hash::<H, F, D>(builder)
    }
}
impl CreatableTarget for QEDCheckpointGlobalStateRootsGadget {
    fn create_virtual<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let contract_tree_root = builder.add_virtual_hash();
        let deposit_tree_root = builder.add_virtual_hash();
        let user_tree_root = builder.add_virtual_hash();
        let withdrawal_tree_root = builder.add_virtual_hash();
        
        Self {
            contract_tree_root,
            deposit_tree_root,
            user_tree_root,
            withdrawal_tree_root,
        }
        
    }
}
impl ToTargets for QEDCheckpointGlobalStateRootsGadget {
    fn to_targets(&self) -> Vec<Target> {
        vec![
            self.contract_tree_root.elements[0],
            self.contract_tree_root.elements[1],
            self.contract_tree_root.elements[2],
            self.contract_tree_root.elements[3],

            self.deposit_tree_root.elements[0],
            self.deposit_tree_root.elements[1],
            self.deposit_tree_root.elements[2],
            self.deposit_tree_root.elements[3],

            self.user_tree_root.elements[0],
            self.user_tree_root.elements[1],
            self.user_tree_root.elements[2],
            self.user_tree_root.elements[3],

            self.withdrawal_tree_root.elements[0],
            self.withdrawal_tree_root.elements[1],
            self.withdrawal_tree_root.elements[2],
            self.withdrawal_tree_root.elements[3],
        ]
    }
}
impl FromTargets for QEDCheckpointGlobalStateRootsGadget {
    fn from_targets(targets: &[Target]) -> Self {
        if targets.len() != 16 {
            panic!("tried to create QEDCheckpointGlobalStateRootsGadget from an array of {} targets, but expected an array of 16 targets", targets.len());
        }
        let contract_tree_root = HashOutTarget {
            elements: [
                targets[0],
                targets[1],
                targets[2],
                targets[3],
            ]
        };
        let deposit_tree_root = HashOutTarget {
            elements: [
                targets[4],
                targets[5],
                targets[6],
                targets[7],
            ]
        };
        let user_tree_root = HashOutTarget {
            elements: [
                targets[8],
                targets[9],
                targets[10],
                targets[11],
            ]
        };
        let withdrawal_tree_root = HashOutTarget {
            elements: [
                targets[12],
                targets[13],
                targets[14],
                targets[15],
            ]
        };
        Self {
            contract_tree_root,
            deposit_tree_root,
            user_tree_root,
            withdrawal_tree_root,
        }
    }
}


impl<F: RichField> WitnessValueFor<QEDCheckpointGlobalStateRootsGadget, F, true> for QEDCheckpointGlobalStateRoots<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDCheckpointGlobalStateRootsGadget) -> anyhow::Result<()> {
        target.set_witness(witness, self)
    }
}

impl<F: RichField> WitnessValueFor<QEDCheckpointGlobalStateRootsGadget, F, false> for QEDCheckpointGlobalStateRoots<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDCheckpointGlobalStateRootsGadget) -> anyhow::Result<()> {
        target.set_witness(witness, self)
    }
}



