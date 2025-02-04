use anyhow::Ok;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::hash_types::{HashOut, RichField},
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::{merkle::core::{DeltaMerkleProofCore, MerkleProofCore}, traits::qhashable::QFieldHashable, utils::safe_hash_fixed_length};
use qed_data::dpn::{cfc_context_input::{DapenCFCUserTransactionEndContext, DapenCFCUserTransactionInputContext}, proving_session::DPNProvingSessionSimpleMethodCall};
use qed_store::{
    config::store_config::QEDHasher, controllers::local::proving_session::QEDLocalProvingSessionStore, store::imm::{cmd::{QSRMerkleCmd, QSRMerkleCmdGetUserContractStateTreeMerkleProof, QSRMerkleCmdGetUserContractTreeMerkleProof}, cmd_processor::{DPNInvokeDeferredMethodCallWitness, DPNReadOtherUserContractStateLeafMerkleProof, DPNStateCmdWitness, QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}}
};
use qedlang_core::dpn::{
    ops::{
        op_types::DPNOpType,
        state_cmd::{data::DPNStateCmd, types::DPNStateCmdCore},
    },
    vm::{def::DPNFunctionCircuitDefinition, exec::SimpleDPNExecutor},
};
use serde::{Deserialize, Serialize};

use super::cfc_input::DapenContractFunctionCircuitInput;
fn mp_to_dmp<H: PartialEq + Copy>(mp: MerkleProofCore<H>) -> DeltaMerkleProofCore<H> {
    DeltaMerkleProofCore {
        old_root: mp.root,
        old_value: mp.value,
        new_root: mp.root,
        new_value: mp.value,
        index: mp.index,
        siblings: mp.siblings,
    }
}
pub trait QEDCmdInputWitnessResolver<F: RichField> {
    fn resolve_vec(
        &mut self,
        state_cmd: &DPNStateCmd<u64>,
    ) -> anyhow::Result<QEDCmdWithInputAndWitness<F>>;
}

impl<R: QEDReadCommandProcessorSync<GF>> QEDCmdInputWitnessResolver<GF>
    for QEDLocalProvingSessionStore<GF, R>
{
    fn resolve_vec(
        &mut self,
        state_cmd: &DPNStateCmd<u64>,
    ) -> anyhow::Result<QEDCmdWithInputAndWitness<GF>> {
        let current_contract_id = self.get_current_contract_id();
        match state_cmd {
            DPNStateCmd::SetContractStateSlotHash(c) => {
                if c.condition == 0 {
                    let mp = self.get_contract_state_slot(
                        current_contract_id,
                        GoldilocksField::from_noncanonical_u64(c.slot_index),
                    )?;
                    let dmp = mp_to_dmp(mp);
                    let result = dmp.new_value.0.elements.to_vec();
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);

                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                } else {
                    let dmp = self.set_contract_state_slot(
                        current_contract_id,
                        GF::from_canonical_u64(c.slot_index),
                        QHashOut::from_values(c.value[0], c.value[1], c.value[2], c.value[3]),
                    )?;
                    let result = dmp.new_value.0.elements.to_vec();
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                }
            }
            DPNStateCmd::SetContractStateSlotSingle(c) => {
                let slot_index = GF::from_canonical_u64(c.sub_slot_index / 4u64);
                let n = (c.sub_slot_index & 0b11) as usize;
                let mp = self.get_contract_state_slot(current_contract_id, slot_index)?;

                let cur = mp.value.0.elements;
                if c.condition == 0 {
                    let dmp = mp_to_dmp(mp);

                    let result = vec![GF::from_canonical_u64(c.value)];
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                } else {
                    let mut new_elements = cur.clone();
                    new_elements[n] = GF::from_canonical_u64(c.value);

                    let dmp = self.set_contract_state_slot(
                        current_contract_id,
                        slot_index,
                        QHashOut(HashOut {
                            elements: new_elements,
                        }),
                    )?;
                    let result = vec![GF::from_canonical_u64(c.value)];
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                }
            }
            DPNStateCmd::SetContractStateSlotRange(c) => {
                if c.condition == 0 {
                    let r = self.resolve_vec(&DPNStateCmd::get_self_user_current_contract_state_slot_range(c.sub_slot_index, c.value.len() as u32))?;
                    match r.witness {
                        DPNStateCmdWitness::MerkleProofArray(vec) => {
                            let dmp = vec
                                .iter()
                                .map(|x| mp_to_dmp(x.clone()))
                                .collect::<Vec<_>>();
                            let result = c
                                .value
                                .iter()
                                .map(|x| GF::from_canonical_u64(*x))
                                .collect::<Vec<GF>>();
                            let witness = DPNStateCmdWitness::DeltaMerkleProofArray(dmp);
                            return Ok(QEDCmdWithInputAndWitness {
                                state_cmd: state_cmd.clone(),
                                witness,
                                result,
                            });
                        },
                        _ => panic!("invalid response type witness for get contrac state range")
                    }
                }
                if c.value.len() == 1 {
                    let slot_index = GF::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let cur = self
                        .get_contract_state_slot(current_contract_id, slot_index)?
                        .value
                        .0
                        .elements;
                    let mut new_elements = cur.clone();
                    new_elements[n] = GF::from_canonical_u64(c.value[0]);

                    let dmp = self.set_contract_state_slot(
                        current_contract_id,
                        slot_index,
                        QHashOut(HashOut {
                            elements: new_elements,
                        }),
                    )?;
                    let result = vec![GF::from_canonical_u64(c.value[0])];
                    let witness = DPNStateCmdWitness::DeltaMerkleProofArray(vec![dmp]);
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                } else {
                    let base_offset = c.sub_slot_index % 4u64;
                    let end_sub_index = (c.value.len() as u64) + c.sub_slot_index;
                    let end_offset = end_sub_index % 4u64;
                    let slot_index = c.sub_slot_index / 4u64;
                    let pre_pad_left = base_offset as usize;
                    let post_pad_right = 4 - (end_offset as usize);
                    let end_slot_index = end_sub_index / 4u64;
                    let left_values = self
                        .get_contract_state_slot(
                            current_contract_id,
                            GF::from_canonical_u64(slot_index),
                        )?
                        .value
                        .0
                        .elements;
                    let right_values = self
                        .get_contract_state_slot(
                            current_contract_id,
                            GF::from_canonical_u64(end_slot_index),
                        )?
                        .value
                        .0
                        .elements;
                    let finished_values = vec![
                        left_values[0..pre_pad_left].to_vec(),
                        c.value
                            .to_vec()
                            .iter()
                            .map(|x| GF::from_noncanonical_u64(*x))
                            .collect::<Vec<GF>>(),
                        right_values[post_pad_right..].to_vec(),
                    ]
                    .concat();
                    let r = finished_values
                        .chunks_exact(4)
                        .enumerate()
                        .map(|(i, x)| {
                            self.set_contract_state_slot(
                                current_contract_id,
                                GF::from_canonical_u64((i as u64) + slot_index),
                                QHashOut(HashOut {
                                    elements: [x[0], x[1], x[2], x[3]],
                                }),
                            )
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?;

                    let witness = DPNStateCmdWitness::DeltaMerkleProofArray(r);
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result: c
                            .value
                            .iter()
                            .map(|x| GF::from_noncanonical_u64(*x))
                            .collect::<Vec<GF>>(),
                    })
                }
            }
            DPNStateCmd::InvokeExternalContractFunctionSync(_c) => todo!(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => {
                let witness = self.get_contract_state_slot(
                    current_contract_id,
                    GF::from_canonical_u64(c.slot_index),
                )?;
                Ok(QEDCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: witness.value.0.elements.to_vec(),
                    witness: DPNStateCmdWitness::MerkleProof(witness),
                })
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => {
                let slot_index = GF::from_canonical_u64(c.sub_slot_index / 4u64);
                let slot_offset = c.sub_slot_index % 4u64;
                let witness = self.get_contract_state_slot(current_contract_id, slot_index)?;
                Ok(QEDCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: vec![witness.value.0.elements[slot_offset as usize]],
                    witness: DPNStateCmdWitness::MerkleProof(witness),
                })
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => {
                if c.length == 1 {
                    let slot_index = GF::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let cur = self.get_contract_state_slot(current_contract_id, slot_index)?;
                    let el = cur.value.0.elements[n];
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: vec![el],
                        witness: DPNStateCmdWitness::MerkleProofArray(vec![cur]),
                    })
                } else {
                    let base_offset = c.sub_slot_index % 4u64;
                    let end_sub_index = (c.length as u64) + c.sub_slot_index;
                    let end_offset = end_sub_index % 4u64;
                    let slot_index = c.sub_slot_index / 4u64;
                    //let pre_pad_left = base_offset as usize;
                    //let post_pad_right = 4-(end_offset as usize);
                    let end_slot_index = end_sub_index / 4u64;
                    let mut mps = Vec::<MerkleProofCore<QHashOut<GF>>>::new();
                    let mut result = Vec::<GF>::with_capacity(c.length as usize);
                    for i in slot_index..end_slot_index {
                        let mp = self.get_contract_state_slot(
                            current_contract_id,
                            GF::from_canonical_u64(i),
                        )?;
                        if base_offset != 0 && i == slot_index {
                            result
                                .extend_from_slice(&mp.value.0.elements[(base_offset as usize)..]);
                        }
                        mps.push(mp);
                    }
                    if end_offset != 0 {
                        let mp = self.get_contract_state_slot(
                            current_contract_id,
                            GF::from_canonical_u64(end_slot_index),
                        )?;
                        result.extend_from_slice(&mp.value.0.elements[..(end_offset as usize)]);
                        mps.push(mp);
                    }
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::MerkleProofArray(mps),
                    })
                }
            }
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => {
                let contract_id = GF::from_noncanonical_u64(c.contract_id);
                
                let uct_witness_upper = self.get_self_user_contract_tree_leaf(contract_id)?;
                
                let state_slot_witness_lower = self.get_contract_state_slot(
                    contract_id,
                    GF::from_canonical_u64(c.slot_index),
                )?;
                Ok(QEDCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: state_slot_witness_lower.value.0.elements.to_vec(),
                    witness: DPNStateCmdWitness::MerkleProofArray(vec![
                        uct_witness_upper,
                        state_slot_witness_lower,
                    ]),
                })
            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => {
                let slot_index = GF::from_canonical_u64(c.sub_slot_index / 4u64);
                let slot_offset = c.sub_slot_index % 4u64;
                let contract_id = GF::from_noncanonical_u64(c.contract_id);

                let uct_witness_upper = self.get_self_user_contract_tree_leaf(contract_id)?;
                let state_slot_witness_lower = self.get_contract_state_slot(contract_id, slot_index)?;

                Ok(QEDCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: vec![state_slot_witness_lower.value.0.elements[slot_offset as usize]],
                    witness: DPNStateCmdWitness::MerkleProofArray(vec![
                        uct_witness_upper,
                        state_slot_witness_lower,
                    ]),
                })
            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => {

                let contract_id = GF::from_noncanonical_u64(c.contract_id);

                let uct_witness_upper = self.get_self_user_contract_tree_leaf(contract_id)?;

                if c.length == 1 {
                    let slot_index = GF::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let cur = self.get_contract_state_slot(contract_id, slot_index)?;
                    let el = cur.value.0.elements[n];
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: vec![el],
                        witness: DPNStateCmdWitness::MerkleProofArray(vec![uct_witness_upper, cur]),
                    })
                } else {
                    let base_offset = c.sub_slot_index % 4u64;
                    let end_sub_index = (c.length as u64) + c.sub_slot_index;
                    let end_offset = end_sub_index % 4u64;
                    let slot_index = c.sub_slot_index / 4u64;
                    //let pre_pad_left = base_offset as usize;
                    //let post_pad_right = 4-(end_offset as usize);
                    let end_slot_index = end_sub_index / 4u64;
                    let mut mps = vec![uct_witness_upper];
                    let mut result = Vec::<GF>::with_capacity(c.length as usize);
                    for i in slot_index..end_slot_index {
                        let mp = self.get_contract_state_slot(
                            contract_id,
                            GF::from_canonical_u64(i),
                        )?;
                        if base_offset != 0 && i == slot_index {
                            result
                                .extend_from_slice(&mp.value.0.elements[(base_offset as usize)..]);
                        }
                        mps.push(mp);
                    }
                    if end_offset != 0 {
                        let mp = self.get_contract_state_slot(
                            contract_id,
                            GF::from_canonical_u64(end_slot_index),
                        )?;
                        result.extend_from_slice(&mp.value.0.elements[..(end_offset as usize)]);
                        mps.push(mp);
                    }
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::MerkleProofArray(mps),
                    })
                }
            },
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => {
                let user_id = GF::from_noncanonical_u64(c.user_id);

                let user_leaf_witness = self.get_external_user_leaf_proof(user_id)?;
                let contract_state_proof = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(QSRMerkleCmdGetUserContractTreeMerkleProof{
                    checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                    user_id: c.user_id,
                    contract_id: c.contract_id as u32,
                }))?;

                let state_slot_proof = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(QSRMerkleCmdGetUserContractStateTreeMerkleProof{
                    checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                    user_id: c.user_id,
                    contract_id: c.contract_id as u32,
                    height: c.contract_state_tree_height,
                    leaf_id: c.slot_index,
                }))?;
                Ok(QEDCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: state_slot_proof.value.0.elements.to_vec(),
                    witness: DPNStateCmdWitness::ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof{
                        user_leaf_witness,
                        contract_state_proof,
                        state_slot_proofs: vec![state_slot_proof],
                    })
                })
            },
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => {
                let slot_index = GF::from_canonical_u64(c.sub_slot_index / 4u64);
                let slot_offset = c.sub_slot_index % 4u64;
                let user_id = GF::from_noncanonical_u64(c.user_id);

                let user_leaf_witness = self.get_external_user_leaf_proof(user_id)?;
                let contract_state_proof = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(QSRMerkleCmdGetUserContractTreeMerkleProof{
                    checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                    user_id: c.user_id,
                    contract_id: c.contract_id as u32,
                }))?;

                let state_slot_proof = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(QSRMerkleCmdGetUserContractStateTreeMerkleProof{
                    checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                    user_id: c.user_id,
                    contract_id: c.contract_id as u32,
                    height: c.contract_state_tree_height,
                    leaf_id: slot_index.to_canonical_u64(),
                }))?;
                Ok(QEDCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: vec![state_slot_proof.value.0.elements[slot_offset as usize]],
                    witness: DPNStateCmdWitness::ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof{
                        user_leaf_witness,
                        contract_state_proof,
                        state_slot_proofs: vec![state_slot_proof],
                    })
                })
            },
            DPNStateCmd::GetOtherUserContractStateSlotRange(_c) => todo!(),
            DPNStateCmd::InvokeExternalContractFunctionDeferred(c) => {
                let call_data = DPNProvingSessionSimpleMethodCall{
                    contract_id: GF::from_canonical_u64(c.contract_id),
                    method_id: GF::from_canonical_u64(c.method_id),
                    inputs: c.input_args.iter().map(|x| GF::from_canonical_u64(*x)).collect::<Vec<GF>>(),
                };
                if c.condition == 0 {
                    let insertion_proof_placeholder = mp_to_dmp(self.get_latest_deferred_tx_leaf()?);
                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: call_data.qfhash::<QEDHasher>().0.elements.to_vec(),
                        witness: DPNStateCmdWitness::InvokeExternalContractFunctionDeferred(DPNInvokeDeferredMethodCallWitness{
                            call_data,
                            insertion_proof: insertion_proof_placeholder,
                        })
                    })    
                }else{
                    let insertion_proof = self.add_deferred_tx_to_debt(call_data.clone())?;

                    Ok(QEDCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: call_data.qfhash::<QEDHasher>().0.elements.to_vec(),
                        witness: DPNStateCmdWitness::InvokeExternalContractFunctionDeferred(DPNInvokeDeferredMethodCallWitness{
                            call_data,
                            insertion_proof,
                        })
                    })    
                }


            },
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDCmdWithInputAndWitness<F: RichField> {
    pub state_cmd: DPNStateCmd<u64>,
    pub witness: DPNStateCmdWitness<F>,
    pub result: Vec<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDEvalSessionResult<F: RichField> {
    //output: Vec<F>,
    pub cmd_witnesses: Vec<QEDCmdWithInputAndWitness<F>>,
}

impl<F: RichField> QEDEvalSessionResult<F> {
    pub fn new() -> Self {
        Self {
            cmd_witnesses: Vec::new(),
        }
    }
}
type GF = GoldilocksField;
impl QEDEvalSessionResult<GF> {
    pub fn process_state_cmd<R: QEDReadCommandProcessorSync<GF>>(
        &mut self,
        executor: &mut SimpleDPNExecutor<GF>,
        sesh: &mut QEDLocalProvingSessionStore<GF, R>,
        cmd: &DPNStateCmd<u64>,
    ) -> anyhow::Result<()> {
        let real_inputs = cmd
            .get_inputs()
            .iter()
            .map(|x| executor.resolve_target(*x).to_canonical_u64())
            .collect::<Vec<u64>>();
        let new_cmd = cmd.convert_to_u64(&real_inputs);

        let r = sesh.resolve_vec(&new_cmd)?;
        self.cmd_witnesses.push(r);
        Ok(())
    }


    pub fn exec_contract_call<R: QEDReadCommandProcessorSync<GF>>(
        self,
        sesh: &mut QEDLocalProvingSessionStore<GF, R>,
        contract_id: GF,
        fn_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<GF>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<GF>> {
        sesh.init_transaction(DPNProvingSessionSimpleMethodCall{
            contract_id,
            method_id: GF::from_canonical_u32(fn_def.method_id),
            inputs: inputs.clone(),
        })?;
        self.eval_session(fn_def, sesh, inputs)
    }

    fn eval_session<R: QEDReadCommandProcessorSync<GF>>(
        mut self,
        fn_def: &DPNFunctionCircuitDefinition,
        sesh: &mut QEDLocalProvingSessionStore<GF, R>,
        inputs: Vec<GF>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<GF>> {

        let start_session_ctx = sesh.get_fresh_start_ctx_for_user(sesh.get_current_user_id())?;
        let call_data_ctx = sesh.get_call_start_data(sesh.get_current_contract_id(), GF::from_canonical_u32(fn_def.method_id), &inputs)?;

        let inputs_clone = inputs.clone();
        let mut executor = SimpleDPNExecutor::<GF>::new_with_contract_ctx(
            inputs,
            sesh.get_current_user_id(),
            sesh.get_current_contract_id(),
            sesh.get_current_start_checkpoint_id(),
            sesh.get_nonce(),
        );
        let state_cmd_len = fn_def.state_command_resolution_indices.len();
        let mut next_state_cmd_id = 0;
        let mut next_state_cmd_index = if state_cmd_len == 0 {
            fn_def.definitions.len() + 10
        } else {
            fn_def.state_command_resolution_indices[0]
        };
        for (i, def) in fn_def.definitions.iter().enumerate() {
            if def.op_type.eq(&DPNOpType::GetStateCommandResultSingle) {
                let ind = def.inputs[0] as usize;
                executor.push_external_target(self.cmd_witnesses[ind].result[0]);
            } else if def.op_type.eq(&DPNOpType::GetStateCommandResultArray) {
                let ind = def.inputs[0] as usize;
                executor.push_external_target_array(self.cmd_witnesses[ind].result.clone());
            } else if def.op_type.eq(&DPNOpType::GetStateCommandResultHash) {
                let ind = def.inputs[0] as usize;
                executor.push_external_hash([
                    self.cmd_witnesses[ind].result[0],
                    self.cmd_witnesses[ind].result[1],
                    self.cmd_witnesses[ind].result[2],
                    self.cmd_witnesses[ind].result[3],
                ]);
            } else {
                executor.process_var_def(&def);
            }
            while (i + 1) >= next_state_cmd_index {
                self.process_state_cmd(
                    &mut executor,
                    sesh,
                    &fn_def.state_commands[next_state_cmd_id],
                )?;
                next_state_cmd_id += 1;
                if next_state_cmd_id >= state_cmd_len {
                    next_state_cmd_index = fn_def.definitions.len() + 10;
                } else {
                    next_state_cmd_index =
                        fn_def.state_command_resolution_indices[next_state_cmd_id];
                }
            }
        }
        for assertion in fn_def.assertions.iter() {
            let left = executor.resolve_target(assertion.left).to_canonical_u64();
            let right = executor.resolve_target(assertion.right).to_canonical_u64();
            if left != right {
                anyhow::bail!(
                    "assertion failed: {} (left: {}, right: {})",
                    assertion.message,
                    left,
                    right
                );
            }
        }
        let outputs = fn_def
            .circuit_outputs
            .iter()
            .map(|x| executor.resolve_target(*x))
            .collect::<Vec<GF>>();
        let end_ctx = DapenCFCUserTransactionEndContext{
            end_contract_state_tree_root: sesh.get_contract_state_slot(sesh.get_current_contract_id(), GF::ZERO)?.root,
            end_deferred_tx_debt_tree_root: sesh.get_latest_deferred_tx_leaf()?.root,
            outputs_hash: safe_hash_fixed_length::<QEDHasher, GF>(&outputs),
            outputs_length: GF::from_noncanonical_u64(outputs.len() as u64),
            total_events_emitted: GF::from_noncanonical_u64(0),
            total_balance_spent: GF::from_noncanonical_u64(0),
        };
        let input_ctx = DapenCFCUserTransactionInputContext{
            proving_session_start_ctx: start_session_ctx,
            transaction_call_start_ctx: call_data_ctx,
            transaction_end_ctx: end_ctx,
        };

        sesh.finalize_transaction()?;

        
        Ok(DapenContractFunctionCircuitInput{
            inputs: inputs_clone,
            outputs,
            cmd_witnesses: self.cmd_witnesses,
            session_proof_tree_root: sesh.get_q_recursion_proof_tree_root(),
            tx_input_ctx: input_ctx,
        })
    }
}
