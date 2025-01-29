use plonky2::{hash::hash_types::RichField, iop::witness::Witness};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore};
use qed_data::qdata::user::QEDUserLeaf;
use qed_exec::vm::{cfc_input::DapenContractFunctionCircuitInput, exec::QEDCmdWithInputAndWitness};
use qedlang_core::dpn::{ops::state_cmd::data::DPNStateCmd, vm::def::DPNFunctionCircuitDefinition};

use super::state_readers::{CKInvokeDeferredMethodCall, StateCommandCacheKey, StateReaderGadget, StateReaderReferenceKeyType};

/* 
fn some_or_error<T>(v: Option<T>) -> anyhow::Result<T> {
    match v {
        Some(x) => Ok(x),
        None => anyhow::bail!("unwrapped None value"),
    }
}
*/

#[derive(Clone, Debug, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct StateReaderGadgetWitnessBuilderState {
    pub contract_call_epoch: u32,
    pub deferred_tx_count: u32,
    pub write_epoch: u32,
}
impl StateReaderGadgetWitnessBuilderState {
    pub fn new() -> Self {
        Self{
            contract_call_epoch: 0,
            deferred_tx_count: 0,
            write_epoch: 0,
        }
    }
    pub fn inc_contract_call_epoch(&mut self) -> u32 {
        self.contract_call_epoch += 1;
        self.contract_call_epoch
    }
    pub fn inc_deferred_tx_count(&mut self) -> u32 {
        self.deferred_tx_count += 1;
        self.deferred_tx_count
    }
    pub fn inc_write_epoch(&mut self) -> u32 {
        self.write_epoch += 1;
        self.write_epoch
    }
}
// witness handlers
impl StateReaderGadget {
    fn set_witness_for_key_dmp<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        ck: &StateCommandCacheKey,
        witness_value: &DeltaMerkleProofCore<QHashOut<F>>,
    ) -> anyhow::Result<()> {
        let reader_ref_key = self.gadget_map.get(ck);
        if reader_ref_key.is_some() {
            let reader_ref_key = reader_ref_key.unwrap().to_owned();
            match reader_ref_key.gadget_type {
                StateReaderReferenceKeyType::DeltaMerkleProof => {
                    self.delta_merkle_proofs[reader_ref_key.gadget_index].set_witness::<W, F>(
                        witness, 
                        F::from_noncanonical_u64(witness_value.index), 
                        witness_value.old_value, 
                        witness_value.new_value, 
                        &witness_value.siblings
                    );
                },
                v => anyhow::bail!("set_witness_for_key_dmp expects to set the witness for a DeltaMerkleMerkleProof gadget, but got {:?}", v)
            }
        }
        Ok(())
    }
    fn set_witness_for_key_mp<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        ck: &StateCommandCacheKey,
        witness_value: &MerkleProofCore<QHashOut<F>>,
    ) -> anyhow::Result<()> {
        let reader_ref_key = self.gadget_map.get(ck);
        if reader_ref_key.is_some() {
            let reader_ref_key = reader_ref_key.unwrap().to_owned();
            match reader_ref_key.gadget_type {
                StateReaderReferenceKeyType::MerkleProof => {
                    self.merkle_proofs[reader_ref_key.gadget_index].set_witness_generic::<W, F>(
                        witness, 
                        F::from_noncanonical_u64(witness_value.index), 
                        witness_value.value, 
                        &witness_value.siblings,
                    );
                },
                v => anyhow::bail!("set_witness_for_key_dmp expects to set the witness for a DeltaMerkleMerkleProof gadget, but got {:?}", v)
            }
        }
        Ok(())
    }
    fn set_witness_for_key_user_leaf<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        ck: &StateCommandCacheKey,
        witness_value: &QEDUserLeaf<F>,
    ) -> anyhow::Result<()> {
        let reader_ref_key = self.gadget_map.get(ck);
        if reader_ref_key.is_some() {
            let reader_ref_key = reader_ref_key.unwrap().to_owned();
            match reader_ref_key.gadget_type {
                StateReaderReferenceKeyType::UserLeaf => {
                    self.user_leaves[reader_ref_key.gadget_index].set_witness(
                        witness,
                        witness_value,
                    );
                },
                v => anyhow::bail!("set_witness_for_key_user_leaf expects to set the witness for a UserLeaf gadget, but got {:?}", v)
            }
        }
        Ok(())
    }
    fn set_witness_single<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        def_cmd: &DPNStateCmd<u64>,
        cmd_witness: &QEDCmdWithInputAndWitness<F>,
        wb_state: &mut StateReaderGadgetWitnessBuilderState,
    ) -> anyhow::Result<()>{
        match &def_cmd {
            DPNStateCmd::SetContractStateSlotHash(c) => {
                let ck = StateCommandCacheKey::new_write_current_contract_slot(
                    c.slot_index,
                    c.condition,
                    wb_state.write_epoch,
                );
                self.set_witness_for_key_dmp(
                    witness, 
                    &ck, 
                    cmd_witness.witness.get_delta_merkle_proof_ref()
                )?;
                wb_state.inc_write_epoch();
            }
            DPNStateCmd::SetContractStateSlotSingle(c) => {
                let ck = StateCommandCacheKey::new_write_current_contract_single(
                    c.sub_slot_index,
                    c.condition,
                    wb_state.write_epoch,
                );
                self.set_witness_for_key_dmp(
                    witness, 
                    &ck, 
                    cmd_witness.witness.get_delta_merkle_proof_ref()
                )?;
                wb_state.inc_write_epoch();
            }
            DPNStateCmd::SetContractStateSlotRange(_c) => todo!(),
            DPNStateCmd::InvokeExternalContractFunctionSync(_c) => todo!(),
            DPNStateCmd::InvokeExternalContractFunctionDeferred(c) => {
                let ck = StateCommandCacheKey::InvokeDeferredMethodCall(CKInvokeDeferredMethodCall::new(
                    c.condition, 
                    c.contract_id, 
                    c.method_id, 
                    wb_state.deferred_tx_count,
                    &c.input_args,
                ));
                self.set_witness_for_key_dmp(
                    witness, 
                    &ck, 
                    cmd_witness.witness.get_delta_merkle_proof_ref()
                )?;
                wb_state.inc_deferred_tx_count();

            },
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => {
                let ck = StateCommandCacheKey::new_read_current_contract_slot(
                    c.slot_index,
                    wb_state.write_epoch,
                );
                self.set_witness_for_key_mp(
                    witness, 
                    &ck,
                    cmd_witness.witness.get_merkle_proof_ref()
                )?;
            },
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => {

                let ck = StateCommandCacheKey::new_read_current_contract_single(
                    c.sub_slot_index,
                    wb_state.write_epoch,
                );
                self.set_witness_for_key_mp(
                    witness, 
                    &ck,
                    cmd_witness.witness.get_merkle_proof_ref(),
                )?;
            },
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(_c) => todo!(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => {

                let read_root_ck = StateCommandCacheKey::new_read_self_user_external_contract_root(
                    c.contract_id,
                    self.contract_call_epoch,
                );

                let proofs = cmd_witness.witness.get_merkle_proof_array_ref();
                self.set_witness_for_key_mp(witness, &read_root_ck, &proofs[0])?;


                let contract_state_tree_ck =
                    StateCommandCacheKey::new_read_self_user_external_contract_slot(
                        c.contract_id,
                        c.slot_index,
                        self.contract_call_epoch,
                    );
                
                self.set_witness_for_key_mp(witness, &contract_state_tree_ck, &proofs[1])?;
            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(_c) => todo!(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(_c) => todo!(),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => {
/* 
        let user_tree_ck =
        StateCommandCacheKey::new_read_other_user_leaf_hash(
            user_target_id
        );
        let user_leaf_ck =
            StateCommandCacheKey::new_read_other_user_leaf(
                user_target_id
            );*/
                let user_target_id = c.user_id;
                let contract_target_id = c.contract_id;
                let slot_target_id = c.slot_index;

                let user_tree_ck = StateCommandCacheKey::new_read_other_user_leaf_hash(
                    user_target_id
                );

                let user_leaf_ck = StateCommandCacheKey::new_read_other_user_leaf(
                    user_target_id
                );

                let uct_ck = StateCommandCacheKey::new_read_other_user_contract_root(
                    user_target_id,
                    contract_target_id,
                );

                let cst_ck = StateCommandCacheKey::new_read_other_user_contract_slot(
                    user_target_id,
                    contract_target_id,
                    slot_target_id,
                );
                let read_witness = cmd_witness.witness.get_read_other_contract_state_ref();

                self.set_witness_for_key_mp(
                    witness, 
                    &user_tree_ck, 
                    &read_witness.user_leaf_witness.user_tree_proof
                )?;
                self.set_witness_for_key_user_leaf(
                    witness, 
                    &user_leaf_ck, 
                    &read_witness.user_leaf_witness.user_leaf
                )?;
                self.set_witness_for_key_mp(
                    witness, 
                    &uct_ck, 
                    &read_witness.contract_state_proof
                )?;
                self.set_witness_for_key_mp(
                    witness, 
                    &cst_ck, 
                    &read_witness.state_slot_proofs[0]
                )?;
            },
            DPNStateCmd::GetOtherUserContractStateSlotSingle(_c) => todo!(),
            DPNStateCmd::GetOtherUserContractStateSlotRange(_c) => todo!(),
        };
        Ok(())
    }
    pub fn set_witness<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        input: &DapenContractFunctionCircuitInput<F>,
        fn_def: &DPNFunctionCircuitDefinition,
    ) {
        let mut wb = StateReaderGadgetWitnessBuilderState::new();

        for (dsc, ciw) in fn_def.state_commands.iter().zip(input.cmd_witnesses.iter()) {
            self.set_witness_single(
                witness,
                 dsc,
                  ciw,
                   &mut wb
            ).unwrap();
        }
    }
}
