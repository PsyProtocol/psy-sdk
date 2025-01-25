use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::{target::Target, witness::Witness}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_common_circuit::traits::{AlgebraicHashableTarget, CreatableTarget, FromTargets, ToTargets, WitnessValueFor};
use qed_data::qdata::contract::QEDContractLeaf;

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct QEDContractLeafGadget {
    pub deployer: HashOutTarget,
    pub function_tree_root: HashOutTarget,
    pub state_tree_height: Target,
}

impl QEDContractLeafGadget {
    pub fn set_witness<F: RichField>(&self, witness: &mut impl Witness<F>, target: &QEDContractLeaf<F>) {
        witness.set_hash_target(self.deployer, target.deployer.0);
        witness.set_hash_target(self.function_tree_root, target.function_tree_root.0);
        witness.set_target(self.state_tree_height, target.state_tree_height);
    }
    pub fn to_hash<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        builder.hash_n_to_hash_no_pad::<H>(self.to_targets())
    }
}
impl AlgebraicHashableTarget for QEDContractLeafGadget {
    fn to_hash_target<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        self.to_hash::<H, F, D>(builder)
    }
}
impl CreatableTarget for QEDContractLeafGadget {
    fn create_virtual<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let deployer = builder.add_virtual_hash();
        let function_tree_root = builder.add_virtual_hash();
        let state_tree_height = builder.add_virtual_target();
        Self {
            deployer,
            function_tree_root,
            state_tree_height,
        }
        
    }
}
impl ToTargets for QEDContractLeafGadget {
    fn to_targets(&self) -> Vec<Target> {
        vec![
            self.deployer.elements[0],
            self.deployer.elements[1],
            self.deployer.elements[2],
            self.deployer.elements[3],
            self.function_tree_root.elements[0],
            self.function_tree_root.elements[1],
            self.function_tree_root.elements[2],
            self.function_tree_root.elements[3],
            self.state_tree_height,
        ]
    }
}
impl FromTargets for QEDContractLeafGadget {
    fn from_targets(targets: &[Target]) -> Self {
        if targets.len() != 9 {
            panic!("tried to create QEDContractLeafGadget from an array of {} targets, but expected an array of 9 targets", targets.len());
        }
        let deployer = HashOutTarget {
            elements: [
                targets[0],
                targets[1],
                targets[2],
                targets[3],
            ]
        };
        let function_tree_root = HashOutTarget {
            elements: [
                targets[4],
                targets[5],
                targets[6],
                targets[7],
            ]
        };
        let state_tree_height = targets [8];
        Self {
            deployer,
            function_tree_root,
            state_tree_height,
        }
    }
}


impl<F: RichField> WitnessValueFor<QEDContractLeafGadget, F, true> for QEDContractLeaf<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDContractLeafGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<QEDContractLeafGadget, F, false> for QEDContractLeaf<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &QEDContractLeafGadget) {
        target.set_witness(witness, self);
    }
}
