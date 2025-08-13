use plonky2::{
    field::extension::Extendable,
    hash::hash_types::RichField,
    iop::witness::Witness,
    plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher},
};
use qed_common_circuit::{builder::connect::CircuitBuilderConnectHelpers, hash::merkle::gadgets::spiderman_append_proof::SpidermanAppendProofGadget, traits::
        CreatableTarget
    }
;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::spiderman::SpidermanUpdateProof;
use qed_data::qdata::contract::QEDContractLeaf;

use crate::gadgets::qdata::contract::QEDContractLeafGadget;


// we keep this separate from DPNProvingSessionCompactMethodCallGadget incase it changes in the future
#[derive(Debug, Clone)]
pub struct BatchDeployContractsGadget {
    pub spiderman_gadget: SpidermanAppendProofGadget,
    pub contract_leaves: Vec<QEDContractLeafGadget>,
}

impl BatchDeployContractsGadget {
    pub fn add_virtual_to<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        contract_tree_height: usize,
        batch_sub_tree_height: usize,
    ) -> Self {
        let top_line_height = contract_tree_height-batch_sub_tree_height;
        let spiderman_gadget = SpidermanAppendProofGadget::add_virtual_to::<H,F,D>(
            builder,
            top_line_height,
            batch_sub_tree_height,
        );

        let total_leaves = 1usize<<batch_sub_tree_height;
        let contract_leaves = (0..total_leaves).map(|_| {
            QEDContractLeafGadget::create_virtual(builder)
        }).collect::<Vec<_>>();



        // for all the newly added contract leaves, ensure their hashes correspond to our append spiderman tree proof
        for (i, (leaf, is_added) )in contract_leaves.iter().zip(spiderman_gadget.get_added_leaves().iter()).enumerate() {
            let contract_leaf_hash = leaf.to_hash::<H,F,D>(builder);
            eprintln!("DEBUGPRINT[879]: deploy_contract.rs:48: spiderman_gadget={:#?}", spiderman_gadget);
            eprintln!("DEBUGPRINT[878]: deploy_contract.rs:47: contract_leaf_hash={:#?}", contract_leaf_hash);
            builder.connect_hashes_if_true(
                *is_added,
                contract_leaf_hash,
                spiderman_gadget.web_proof.new_leaves[i],
            );
        }


        Self {
            spiderman_gadget,
            contract_leaves,
        }
    }
    pub fn set_witness_params<F: RichField>(
        &self,
        witness: &mut impl Witness<F>,
        spiderman_append_proof: &SpidermanUpdateProof<QHashOut<F>>,
        contract_leaves: &[QEDContractLeaf<F>],
    ) -> anyhow::Result<()> {
        eprintln!("DEBUGPRINT[880]: deploy_contract.rs:69: spiderman_append_proof={:#?}", spiderman_append_proof);
        eprintln!("DEBUGPRINT[881]: deploy_contract.rs:70: contract_leaves={:#?}", contract_leaves);
        self.spiderman_gadget.set_witness(witness, spiderman_append_proof)?;
        for (g, v) in self.contract_leaves.iter().zip(contract_leaves.iter()) {
            g.set_witness(witness, v)?;
        }
        if contract_leaves.len() < self.contract_leaves.len() {
            let empty = QEDContractLeaf::default();
            for i in contract_leaves.len()..self.contract_leaves.len() {
                self.contract_leaves[i].set_witness(witness, &empty)?;
            }
        }
        Ok(())
    }
}
