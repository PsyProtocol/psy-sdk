use anyhow::Ok;
use plonky2::{
    field::types::{Field, PrimeField64},
    hash::hash_types::{HashOut, RichField},
};
use psy_common::{data::qhashout::QHashOut, traits::to_qfelts::ToQFelts};
use psy_config::network_constants::DEFAULT_CALLER_CONTRACT_ID_U64;
use psy_crypto::hash::{
    merkle::core::{DeltaMerkleProofCore, MerkleProofCore},
    traits::{
        hasher::{FieldQHasher, MerkleZeroHasherWithMarkedLeaf},
        qhashable::QFieldHashable,
    },
    utils::safe_hash_fixed_length,
};
use psy_data::{
    config::store_config::{PsyFelt, PsyHasher},
    dpn::{
        cfc_context_input::{DapenCFCUserTransactionEndContext, DapenCFCUserTransactionInputContext},
        event::PsyUserEventRecord,
        proving_session::DPNProvingSessionSimpleMethodCall,
    },
    qstore::{
        controllers::proving_session::{
            PsyEventsStore, PsyLocalProvingSessionStore, PsyReadLocalProvingSessionStore, PsyReadLocalProvingSessionStoreMut,
        },
        imm::{
            cmd::{
                QSRCmdGetCheckpointLeafData, QSRCmdGetContractLeafData, QSRMerkleCmd, QSRMerkleCmdGetCheckpointTreeMerkleProof,
                QSRMerkleCmdGetContractTreeMerkleProof, QSRMerkleCmdGetUserContractStateTreeMerkleProof, QSRMerkleCmdGetUserContractTreeMerkleProof,
            },
            cmd_processor::{
                DPNCheckpointLeafStatsWitness, DPNClearEntireTreeWitness, DPNContractLeafWitness, DPNInvokeDeferredMethodCallWitness,
                DPNReadOtherUserContractStateLeafMerkleProof, DPNStateCmdWitness, PsyReadCommandProcessorSync, PsyReadCommandProcessorSyncMut,
            },
        },
    },
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::cfc_input::DapenContractFunctionCircuitInput;
use crate::dpn::{
    ops::{
        op_types::{DPNEventRecord, DPNOpType},
        state_cmd::{data::DPNStateCmd, types::DPNStateCmdCore},
    },
    vm::{def::DPNFunctionCircuitDefinition, exec::SimpleDPNExecutor},
};
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

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub trait PsyCmdInputWitnessResolver<F: RichField + PrimeField64, H: MerkleZeroHasherWithMarkedLeaf<QHashOut<F>> + FieldQHasher<F> + Send> {
    async fn resolve_vec(&mut self, state_cmd: &DPNStateCmd<u64>) -> anyhow::Result<PsyCmdWithInputAndWitness<F>>;
}
//(sub_slot_length-2)%4
/*
const SLOT_MASK_TABLE: [[u8; 4]; 7] = [
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [1, 0, 0, 0],
    [1, 1, 0, 0],
    [1, 1, 1, 0],
    [1, 1, 1, 1],
];

fn get_slot_mask(length: u64, sub_slot_index: u64) -> [u8; 4] {
    let length_minus_2 = length - 2;

    let length_minus_2_low_bits = length_minus_2 & 0b11;
    let sub_slot_index_low_bits = sub_slot_index & 0b11;

    SLOT_MASK_TABLE[(length_minus_2_low_bits + sub_slot_index_low_bits) as usize]
}
*/

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<
        F: RichField + PrimeField64,
        H: MerkleZeroHasherWithMarkedLeaf<QHashOut<F>> + FieldQHasher<F> + Send,
        R: PsyReadCommandProcessorSync<F> + psy_data::qstore::imm::cmd_processor::QUserIdManager + Send + Sync,
    > PsyCmdInputWitnessResolver<F, H> for PsyLocalProvingSessionStore<F, R, H>
{
    async fn resolve_vec(&mut self, state_cmd: &DPNStateCmd<u64>) -> anyhow::Result<PsyCmdWithInputAndWitness<F>> {
        tracing::debug!("Resolving state command: {:#?}", state_cmd);
        let current_contract_id = self.get_current_contract_id();
        match state_cmd {
            DPNStateCmd::SetContractStateSlotHash(c) => {
                if c.condition == 0 {
                    let mp = self
                        .get_contract_state_slot(current_contract_id, F::from_noncanonical_u64(c.slot_index))
                        .await?;
                    let dmp = mp_to_dmp(mp);
                    let result = dmp.new_value.0.elements.to_vec();
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                } else {
                    let dmp = self
                        .set_contract_state_slot(
                            current_contract_id,
                            F::from_canonical_u64(c.slot_index),
                            QHashOut::from_values(c.value[0], c.value[1], c.value[2], c.value[3]),
                        )
                        .await?;
                    let result = dmp.new_value.0.elements.to_vec();
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                }
            }
            DPNStateCmd::SetContractStateSlotSingle(c) => {
                let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                let n = (c.sub_slot_index & 0b11) as usize;
                let mp = self.get_contract_state_slot(current_contract_id, slot_index).await?;

                let cur = mp.value.0.elements;
                if c.condition == 0 {
                    let dmp = mp_to_dmp(mp);

                    let result = vec![F::from_canonical_u64(c.value)];
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                } else {
                    let mut new_elements = cur.clone();
                    new_elements[n] = F::from_canonical_u64(c.value);

                    let dmp = self
                        .set_contract_state_slot(current_contract_id, slot_index, QHashOut(HashOut { elements: new_elements }))
                        .await?;
                    let result = vec![F::from_canonical_u64(c.value)];
                    let witness = DPNStateCmdWitness::DeltaMerkleProof(dmp);
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                }
            }
            DPNStateCmd::SetContractStateSlotRange(c) => {
                if c.condition == 0 {
                    let r = self
                        .resolve_vec(&DPNStateCmd::get_self_user_current_contract_state_slot_range(
                            c.sub_slot_index,
                            c.value.len() as u32,
                        ))
                        .await?;
                    match r.witness {
                        DPNStateCmdWitness::MerkleProofArray(vec) => {
                            let dmp = vec.iter().map(|x| mp_to_dmp(x.clone())).collect::<Vec<_>>();
                            let result = c.value.iter().map(|x| F::from_canonical_u64(*x)).collect::<Vec<F>>();
                            let witness = DPNStateCmdWitness::DeltaMerkleProofArray(dmp);
                            return Ok(PsyCmdWithInputAndWitness {
                                state_cmd: state_cmd.clone(),
                                witness,
                                result,
                            });
                        }
                        _ => panic!("invalid response type witness for get contract state range"),
                    }
                }
                let value_len = c.value.len();
                if value_len == 1 {
                    let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let cur = self.get_contract_state_slot(current_contract_id, slot_index).await?.value.0.elements;
                    let mut new_elements = cur.clone();
                    new_elements[n] = F::from_canonical_u64(c.value[0]);

                    let dmp = self
                        .set_contract_state_slot(current_contract_id, slot_index, QHashOut(HashOut { elements: new_elements }))
                        .await?;
                    let result = vec![F::from_canonical_u64(c.value[0])];
                    let witness = DPNStateCmdWitness::DeltaMerkleProofArray(vec![dmp]);
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result,
                    })
                } else if value_len < 6 {
                    // two merkle proofs

                    let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let mut proof_0_elements = self.get_contract_state_slot(current_contract_id, slot_index).await?.value.0.elements;
                    let mut proof_1_elements = self
                        .get_contract_state_slot(current_contract_id, slot_index + F::ONE)
                        .await?
                        .value
                        .0
                        .elements;
                    for (i, v) in c.value.iter().enumerate() {
                        let r_ind = n + i;
                        if r_ind < 4 {
                            proof_0_elements[r_ind] = F::from_canonical_u64(*v);
                        } else {
                            proof_1_elements[r_ind - 4] = F::from_canonical_u64(*v);
                        }
                    }
                    let delta_proof_0 = self
                        .set_contract_state_slot(current_contract_id, slot_index, QHashOut(HashOut { elements: proof_0_elements }))
                        .await?;
                    let delta_proof_1 = self
                        .set_contract_state_slot(current_contract_id, slot_index + F::ONE, QHashOut(HashOut { elements: proof_1_elements }))
                        .await?;
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness: DPNStateCmdWitness::DeltaMerkleProofArray(vec![delta_proof_0, delta_proof_1]),
                        result: c.value.iter().map(|x| F::from_noncanonical_u64(*x)).collect(),
                    })
                } else {
                    let start_slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n_proofs = ((value_len + 6) / 4) as u64;
                    let sub_slot_index_mod_4 = c.sub_slot_index % 4;
                    let len_minus_2_mod_4 = (value_len - 2) % 4;
                    //let start_slot = c.sub_slot_index / 4;
                    let mut dmps = Vec::with_capacity(n_proofs as usize);
                    let result = c.value.iter().map(|i| F::from_noncanonical_u64(*i)).collect::<Vec<_>>();

                    let slot_mask_type = sub_slot_index_mod_4 as usize + len_minus_2_mod_4;

                    // handle the first proof special case
                    let main_body_proofs_index_offset = (4 - sub_slot_index_mod_4) as usize;
                    let first_proof_set_elements = if sub_slot_index_mod_4 == 0 {
                        [result[0], result[1], result[2], result[3]]
                    } else {
                        let first_proof_existing_value = self
                            .get_contract_state_slot(current_contract_id, start_slot_index)
                            .await?
                            .value
                            .0
                            .elements;
                        if sub_slot_index_mod_4 == 1 {
                            [first_proof_existing_value[0], result[0], result[1], result[2]]
                        } else if sub_slot_index_mod_4 == 2 {
                            [first_proof_existing_value[0], first_proof_existing_value[1], result[0], result[1]]
                        } else {
                            // }else if sub_slot_index_mod_4 == 3 {
                            [
                                first_proof_existing_value[0],
                                first_proof_existing_value[1],
                                first_proof_existing_value[2],
                                result[0],
                            ]
                        }
                    };
                    let dmp = self
                        .set_contract_state_slot(
                            current_contract_id,
                            start_slot_index,
                            QHashOut(HashOut {
                                elements: first_proof_set_elements,
                            }),
                        )
                        .await?;
                    dmps.push(dmp);

                    // we don't need to get the old values for main body proofs
                    for i in 1..(n_proofs - 1) {
                        let current_value_index = main_body_proofs_index_offset + (i - 1) as usize * 4;

                        let set_value = QHashOut(HashOut {
                            elements: [
                                result[current_value_index],
                                result[current_value_index + 1],
                                result[current_value_index + 2],
                                result[current_value_index + 3],
                            ],
                        });

                        let dmp = self
                            .set_contract_state_slot(current_contract_id, start_slot_index + F::from_canonical_u64(i), set_value)
                            .await?;
                        dmps.push(dmp);
                    }

                    // handle the last proof special case
                    /*

                    const SLOT_MASK_TABLE: [[u8; 4]; 7] = [
                        [0, 0, 0, 0], // type 0
                        [0, 0, 0, 0], // type 1
                        [0, 0, 0, 0], // type 2
                        [1, 0, 0, 0], // type 3
                        [1, 1, 0, 0], // type 4
                        [1, 1, 1, 0], // type 5
                        [1, 1, 1, 1], // type 6
                    ];

                    */

                    let last_proof_value_index = main_body_proofs_index_offset + (n_proofs as usize - 2) * 4;
                    let last_proof_slot_index = start_slot_index + F::from_canonical_u64(n_proofs - 1);
                    if slot_mask_type == 6 {
                        // if mask type is 6, we don't need to check the old value
                        // type 6 => [1, 1, 1, 1],

                        let set_value = QHashOut(HashOut {
                            elements: [
                                result[last_proof_value_index],
                                result[last_proof_value_index + 1],
                                result[last_proof_value_index + 2],
                                result[last_proof_value_index + 3],
                            ],
                        });

                        let dmp = self
                            .set_contract_state_slot(current_contract_id, last_proof_slot_index, set_value)
                            .await?;
                        dmps.push(dmp);
                    } else if slot_mask_type < 3 {
                        let last_proof_existing_mp = self.get_contract_state_slot(current_contract_id, last_proof_slot_index).await?;
                        // type 0, 1, 2 => [0, 0, 0, 0]
                        // if slot mask type is < 3, then we are done and can just trasform the existing
                        // mp into a delta merkle proof
                        dmps.push(last_proof_existing_mp.to_delta_merkle_proof_inplace());
                    } else {
                        // handle types 3, 4, 5
                        // get the previous value of this slot
                        let last_proof_existing_value = self
                            .get_contract_state_slot(current_contract_id, last_proof_slot_index)
                            .await?
                            .value
                            .0
                            .elements;

                        let new_set_value = if slot_mask_type == 3 {
                            // type 3 => [1, 0, 0, 0]
                            [
                                result[last_proof_value_index],
                                last_proof_existing_value[1],
                                last_proof_existing_value[2],
                                last_proof_existing_value[3],
                            ]
                        } else if slot_mask_type == 4 {
                            [
                                result[last_proof_value_index],
                                result[last_proof_value_index + 1],
                                last_proof_existing_value[2],
                                last_proof_existing_value[3],
                            ]
                        } else {
                            // if slot_mask_type == 5 {
                            [
                                result[last_proof_value_index],
                                result[last_proof_value_index + 1],
                                result[last_proof_value_index + 2],
                                last_proof_existing_value[3],
                            ]
                        };
                        let dmp = self
                            .set_contract_state_slot(current_contract_id, last_proof_slot_index, QHashOut(HashOut { elements: new_set_value }))
                            .await?;
                        dmps.push(dmp);
                    }

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::DeltaMerkleProofArray(dmps),
                    })
                    /*
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
                            F::from_canonical_u64(slot_index),
                        )?
                        .value
                        .0
                        .elements;
                    let right_values = self
                        .get_contract_state_slot(
                            current_contract_id,
                            F::from_canonical_u64(end_slot_index),
                        )?
                        .value
                        .0
                        .elements;
                    let finished_values = vec![
                        left_values[0..pre_pad_left].to_vec(),
                        c.value
                            .to_vec()
                            .iter()
                            .map(|x| F::from_noncanonical_u64(*x))
                            .collect::<Vec<F>>(),
                        right_values[post_pad_right..].to_vec(),
                    ]
                    .concat();
                    let r = finished_values
                        .chunks_exact(4)
                        .enumerate()
                        .map(|(i, x)| {
                            self.set_contract_state_slot(
                                current_contract_id,
                                F::from_canonical_u64((i as u64) + slot_index),
                                QHashOut(HashOut {
                                    elements: [x[0], x[1], x[2], x[3]],
                                }),
                            )
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?;

                    let witness = DPNStateCmdWitness::DeltaMerkleProofArray(r);
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        witness,
                        result: c
                            .value
                            .iter()
                            .map(|x| F::from_noncanonical_u64(*x))
                            .collect::<Vec<F>>(),
                    })*/
                }
            }
            DPNStateCmd::InvokeExternalContractFunctionSync(_c) => todo!(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => {
                let witness = self
                    .get_contract_state_slot(current_contract_id, F::from_canonical_u64(c.slot_index))
                    .await?;
                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: witness.value.0.elements.to_vec(),
                    witness: DPNStateCmdWitness::MerkleProof(witness),
                })
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => {
                let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                let slot_offset = c.sub_slot_index % 4u64;
                let witness = self.get_contract_state_slot(current_contract_id, slot_index).await?;
                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: vec![witness.value.0.elements[slot_offset as usize]],
                    witness: DPNStateCmdWitness::MerkleProof(witness),
                })
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => {
                if c.length == 1 {
                    // one merkle proof
                    let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let cur = self.get_contract_state_slot(current_contract_id, slot_index).await?;
                    let el = cur.value.0.elements[n];
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: vec![el],
                        witness: DPNStateCmdWitness::MerkleProofArray(vec![cur]),
                    })
                } else if c.length < 6 {
                    // two merkle proofs

                    let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let proof_0 = self.get_contract_state_slot(current_contract_id, slot_index).await?;
                    let proof_1 = self.get_contract_state_slot(current_contract_id, slot_index + F::ONE).await?;

                    let elements = [proof_0.value.0.elements, proof_1.value.0.elements].concat();

                    let result = elements[n..(n + c.length as usize)].to_vec();
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::MerkleProofArray(vec![proof_0, proof_1]),
                    })
                } else {
                    // max proofs needed = floor((c.length+6)/4)
                    /*
                        The first leaf can always be:
                            if sub_slot_index%4 == 0: 1, 1, 1, 1
                            if sub_slot_index%4 == 1: 0, 1, 1, 1
                            if sub_slot_index%4 == 2: 0, 0, 1, 1
                            if sub_slot_index%4 == 3: 0, 0, 0, 1

                    The last leaf takes on the pattern (where 1 means we modify the element and 0 means we keep it the same):
                        if (length-2)%4 == 0 {
                            if sub_slot_index%4 == 0: 0, 0, 0, 0
                            if sub_slot_index%4 == 1: 0, 0, 0, 0
                            if sub_slot_index%4 == 2: 0, 0, 0, 0
                            if sub_slot_index%4 == 3: 1, 0, 0, 0
                        }
                        ======================================

                        if (length-2)%4 == 1 {
                            if sub_slot_index%4 == 0: 0, 0, 0, 0
                            if sub_slot_index%4 == 1: 0, 0, 0, 0
                            if sub_slot_index%4 == 2: 1, 0, 0, 0
                            if sub_slot_index%4 == 3: 1, 1, 0, 0
                        }
                        ======================================

                        if (length-2)%4 == 2 {
                            if sub_slot_index%4 == 0: 0, 0, 0, 0
                            if sub_slot_index%4 == 1: 1, 0, 0, 0
                            if sub_slot_index%4 == 2: 1, 1, 0, 0
                            if sub_slot_index%4 == 3: 1, 1, 1, 0
                        }
                        ======================================

                        if (length-2)%4 == 3 {
                            if sub_slot_index%4 == 0: 1, 0, 0, 0
                            if sub_slot_index%4 == 1: 1, 1, 0, 0
                            if sub_slot_index%4 == 2: 1, 1, 1, 0
                            if sub_slot_index%4 == 3: 1, 1, 1, 1
                        }
                        ======================================
                     */

                    let n_proofs = ((c.length + 6) / 4) as u64;
                    let sub_slot_index_mod_4 = c.sub_slot_index % 4;
                    let start_slot = c.sub_slot_index / 4;
                    let mut mps = Vec::with_capacity(n_proofs as usize);
                    let mut result = Vec::<F>::with_capacity(c.length as usize);

                    let len_minus_2_mod_4 = (c.length - 2) % 4;

                    for i in 0..n_proofs {
                        let mp = self
                            .get_contract_state_slot(current_contract_id, F::from_canonical_u64(start_slot + i))
                            .await?;
                        if i == 0 {
                            if sub_slot_index_mod_4 == 0 {
                                result.push(mp.value.0.elements[0]);
                                result.push(mp.value.0.elements[1]);
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 1 {
                                result.push(mp.value.0.elements[1]);
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 2 {
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 3 {
                                result.push(mp.value.0.elements[3]);
                            }
                        } else if i == (n_proofs - 1) {
                            let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                            /*

                                const SLOT_MASK_TABLE: [[u8; 4]; 7] = [
                                    [0, 0, 0, 0],
                                    [0, 0, 0, 0],
                                    [0, 0, 0, 0],
                                    [1, 0, 0, 0],
                                    [1, 1, 0, 0],
                                    [1, 1, 1, 0],
                                    [1, 1, 1, 1],
                                ];
                            */
                            if slot_mask_type >= 3 {
                                result.push(mp.value.0.elements[0]);
                            }
                            if slot_mask_type >= 4 {
                                result.push(mp.value.0.elements[1]);
                            }
                            if slot_mask_type >= 5 {
                                result.push(mp.value.0.elements[2]);
                            }
                            if slot_mask_type >= 6 {
                                result.push(mp.value.0.elements[3]);
                            }
                        } else {
                            result.extend_from_slice(&mp.value.0.elements);
                        }
                        mps.push(mp);
                    }

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::MerkleProofArray(mps),
                    })

                    /*
                    let base_offset = c.sub_slot_index % 4u64;
                    let end_sub_index = (c.length as u64) + c.sub_slot_index;
                    let end_offset = end_sub_index % 4u64;
                    let slot_index = c.sub_slot_index / 4u64;
                    //let pre_pad_left = base_offset as usize;
                    //let post_pad_right = 4-(end_offset as usize);
                    let end_slot_index = end_sub_index / 4u64;
                    let mut mps = Vec::<MerkleProofCore<QHashOut<F>>>::new();
                    let mut result = Vec::<F>::with_capacity(c.length as usize);
                    for i in slot_index..end_slot_index {
                        let mp = self.get_contract_state_slot(
                            current_contract_id,
                            F::from_canonical_u64(i),
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
                            F::from_canonical_u64(end_slot_index),
                        )?;
                        result.extend_from_slice(&mp.value.0.elements[..(end_offset as usize)]);
                        mps.push(mp);
                    }
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::MerkleProofArray(mps),
                    })
                    */
                }
            }
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => {
                let contract_id = F::from_noncanonical_u64(c.contract_id);

                let uct_witness_upper = self.get_self_user_contract_tree_leaf(contract_id).await?;

                let state_slot_witness_lower = self.get_contract_state_slot(contract_id, F::from_canonical_u64(c.slot_index)).await?;
                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: state_slot_witness_lower.value.0.elements.to_vec(),
                    witness: DPNStateCmdWitness::MerkleProofArray(vec![uct_witness_upper, state_slot_witness_lower]),
                })
            }
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => {
                let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                let slot_offset = c.sub_slot_index % 4u64;
                let contract_id = F::from_noncanonical_u64(c.contract_id);

                let uct_witness_upper = self.get_self_user_contract_tree_leaf(contract_id).await?;
                let state_slot_witness_lower = self.get_contract_state_slot(contract_id, slot_index).await?;

                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: vec![state_slot_witness_lower.value.0.elements[slot_offset as usize]],
                    witness: DPNStateCmdWitness::MerkleProofArray(vec![uct_witness_upper, state_slot_witness_lower]),
                })
            }
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => {
                let contract_id = F::from_noncanonical_u64(c.contract_id);

                let uct_witness_upper = self.get_self_user_contract_tree_leaf(contract_id).await?;

                if c.length == 1 {
                    let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let cur = self.get_contract_state_slot(contract_id, slot_index).await?;
                    let el = cur.value.0.elements[n];
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: vec![el],
                        witness: DPNStateCmdWitness::MerkleProofArray(vec![uct_witness_upper, cur]),
                    })
                } else if c.length < 6 {
                    // two merkle proofs

                    let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let proof_0 = self.get_contract_state_slot(contract_id, slot_index).await?;
                    let proof_1 = self.get_contract_state_slot(contract_id, slot_index + F::ONE).await?;

                    let elements = [proof_0.value.0.elements, proof_1.value.0.elements].concat();

                    let result = elements[n..(n + c.length as usize)].to_vec();
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::MerkleProofArray(vec![uct_witness_upper, proof_0, proof_1]),
                    })
                } else {
                    let n_proofs = ((c.length + 6) / 4) as u64;
                    let sub_slot_index_mod_4 = c.sub_slot_index % 4;
                    let start_slot = c.sub_slot_index / 4;
                    let mut mps = Vec::with_capacity(n_proofs as usize + 1);
                    let mut result = Vec::<F>::with_capacity(c.length as usize);
                    mps.push(uct_witness_upper);

                    let len_minus_2_mod_4 = (c.length - 2) % 4;

                    for i in 0..n_proofs {
                        let mp = self.get_contract_state_slot(contract_id, F::from_canonical_u64(start_slot + i)).await?;
                        if i == 0 {
                            if sub_slot_index_mod_4 == 0 {
                                result.push(mp.value.0.elements[0]);
                                result.push(mp.value.0.elements[1]);
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 1 {
                                result.push(mp.value.0.elements[1]);
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 2 {
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 3 {
                                result.push(mp.value.0.elements[3]);
                            }
                        } else if i == (n_proofs - 1) {
                            let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                            if slot_mask_type >= 3 {
                                result.push(mp.value.0.elements[0]);
                            }
                            if slot_mask_type >= 4 {
                                result.push(mp.value.0.elements[1]);
                            }
                            if slot_mask_type >= 5 {
                                result.push(mp.value.0.elements[2]);
                            }
                            if slot_mask_type >= 6 {
                                result.push(mp.value.0.elements[3]);
                            }
                        } else {
                            result.extend_from_slice(&mp.value.0.elements);
                        }
                        mps.push(mp);
                    }

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::MerkleProofArray(mps),
                    })
                }
            }
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => {
                let user_id = F::from_noncanonical_u64(c.user_id);

                let user_leaf_witness = self.get_external_user_leaf_proof(user_id).await?;
                let contract_state_proof = self
                    .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(
                        QSRMerkleCmdGetUserContractTreeMerkleProof {
                            checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                            user_id: c.user_id,
                            contract_id: c.contract_id as u32,
                        },
                    ))
                    .await?;

                let state_slot_proof = self
                    .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                        QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                            checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                            user_id: c.user_id,
                            contract_id: c.contract_id as u32,
                            height: c.contract_state_tree_height,
                            leaf_id: c.slot_index,
                        },
                    ))
                    .await?;
                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: state_slot_proof.value.0.elements.to_vec(),
                    witness: DPNStateCmdWitness::ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof {
                        user_leaf_witness,
                        contract_state_proof,
                        state_slot_proofs: vec![state_slot_proof],
                    }),
                })
            }
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => {
                let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                let slot_offset = c.sub_slot_index % 4u64;
                let user_id = F::from_noncanonical_u64(c.user_id);

                let user_leaf_witness = self.get_external_user_leaf_proof(user_id).await?;
                let contract_state_proof = self
                    .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(
                        QSRMerkleCmdGetUserContractTreeMerkleProof {
                            checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                            user_id: c.user_id,
                            contract_id: c.contract_id as u32,
                        },
                    ))
                    .await?;

                let state_slot_proof = self
                    .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                        QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                            checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                            user_id: c.user_id,
                            contract_id: c.contract_id as u32,
                            height: c.contract_state_tree_height,
                            leaf_id: slot_index.to_canonical_u64(),
                        },
                    ))
                    .await?;
                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result: vec![state_slot_proof.value.0.elements[slot_offset as usize]],
                    witness: DPNStateCmdWitness::ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof {
                        user_leaf_witness,
                        contract_state_proof,
                        state_slot_proofs: vec![state_slot_proof],
                    }),
                })
            }
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => {
                let user_id = F::from_noncanonical_u64(c.user_id);

                let user_leaf_witness = self.get_external_user_leaf_proof(user_id).await?;
                let contract_state_proof = self
                    .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractTreeMerkleProof(
                        QSRMerkleCmdGetUserContractTreeMerkleProof {
                            checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                            user_id: c.user_id,
                            contract_id: c.contract_id as u32,
                        },
                    ))
                    .await?;

                if c.length == 1 {
                    //let slot_index = F::from_canonical_u64(c.sub_slot_index / 4u64);
                    //let n = (c.sub_slot_index & 0b11) as usize;

                    let state_slot_proof = self
                        .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                            QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                                checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                                user_id: c.user_id,
                                contract_id: c.contract_id as u32,
                                height: c.contract_state_tree_height,
                                leaf_id: c.sub_slot_index / 4u64,
                            },
                        ))
                        .await?;
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: state_slot_proof.value.0.elements.to_vec(),
                        witness: DPNStateCmdWitness::ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof {
                            user_leaf_witness,
                            contract_state_proof,
                            state_slot_proofs: vec![state_slot_proof],
                        }),
                    })
                } else if c.length < 6 {
                    // two merkle proofs

                    let slot_index = c.sub_slot_index / 4u64;
                    let n = (c.sub_slot_index & 0b11) as usize;
                    let proof_0 = self
                        .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                            QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                                checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                                user_id: c.user_id,
                                contract_id: c.contract_id as u32,
                                height: c.contract_state_tree_height,
                                leaf_id: slot_index,
                            },
                        ))
                        .await?;
                    let proof_1 = self
                        .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                            QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                                checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                                user_id: c.user_id,
                                contract_id: c.contract_id as u32,
                                height: c.contract_state_tree_height,
                                leaf_id: slot_index + 1,
                            },
                        ))
                        .await?;

                    let elements = [proof_0.value.0.elements, proof_1.value.0.elements].concat();

                    let result = elements[n..(n + c.length as usize)].to_vec();

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof {
                            user_leaf_witness,
                            contract_state_proof,
                            state_slot_proofs: vec![proof_0, proof_1],
                        }),
                    })
                } else {
                    let n_proofs = ((c.length + 6) / 4) as u64;
                    let sub_slot_index_mod_4 = c.sub_slot_index % 4;
                    let start_slot = c.sub_slot_index / 4;
                    let mut mps = Vec::with_capacity(n_proofs as usize + 1);
                    let mut result = Vec::<F>::with_capacity(c.length as usize);

                    let len_minus_2_mod_4 = (c.length - 2) % 4;

                    for i in 0..n_proofs {
                        let mp = self
                            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                                QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                                    checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                                    user_id: c.user_id,
                                    contract_id: c.contract_id as u32,
                                    height: c.contract_state_tree_height,
                                    leaf_id: start_slot + i,
                                },
                            ))
                            .await?;
                        if i == 0 {
                            if sub_slot_index_mod_4 == 0 {
                                result.push(mp.value.0.elements[0]);
                                result.push(mp.value.0.elements[1]);
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 1 {
                                result.push(mp.value.0.elements[1]);
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 2 {
                                result.push(mp.value.0.elements[2]);
                                result.push(mp.value.0.elements[3]);
                            } else if sub_slot_index_mod_4 == 3 {
                                result.push(mp.value.0.elements[3]);
                            }
                        } else if i == (n_proofs - 1) {
                            let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                            if slot_mask_type >= 3 {
                                result.push(mp.value.0.elements[0]);
                            }
                            if slot_mask_type >= 4 {
                                result.push(mp.value.0.elements[1]);
                            }
                            if slot_mask_type >= 5 {
                                result.push(mp.value.0.elements[2]);
                            }
                            if slot_mask_type >= 6 {
                                result.push(mp.value.0.elements[3]);
                            }
                        } else {
                            result.extend_from_slice(&mp.value.0.elements);
                        }
                        mps.push(mp);
                    }

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result,
                        witness: DPNStateCmdWitness::ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof {
                            user_leaf_witness,
                            contract_state_proof,
                            state_slot_proofs: mps,
                        }),
                    })
                }
            }
            DPNStateCmd::InvokeExternalContractFunctionDeferred(c) => {
                let call_data = DPNProvingSessionSimpleMethodCall {
                    caller_contract_id: current_contract_id,
                    contract_id: F::from_canonical_u64(c.contract_id),
                    method_id: F::from_canonical_u64(c.method_id),
                    inputs: c.input_args.iter().map(|x| F::from_canonical_u64(*x)).collect::<Vec<F>>(),
                };
                if c.condition == 0 {
                    let insertion_proof_placeholder = mp_to_dmp(self.get_latest_deferred_tx_leaf()?);
                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: call_data
                            .qfhash::<<Self as PsyReadLocalProvingSessionStoreMut<F>>::Hasher>()
                            .0
                            .elements
                            .to_vec(),
                        witness: DPNStateCmdWitness::InvokeExternalContractFunctionDeferred(DPNInvokeDeferredMethodCallWitness {
                            call_data,
                            insertion_proof: insertion_proof_placeholder,
                        }),
                    })
                } else {
                    let insertion_proof = self.add_deferred_tx_to_debt(call_data.clone())?;

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: call_data
                            .qfhash::<<Self as PsyReadLocalProvingSessionStoreMut<F>>::Hasher>()
                            .0
                            .elements
                            .to_vec(),
                        witness: DPNStateCmdWitness::InvokeExternalContractFunctionDeferred(DPNInvokeDeferredMethodCallWitness {
                            call_data,
                            insertion_proof,
                        }),
                    })
                }
            }
            DPNStateCmd::GetContractLeaf(c) => {
                let contract_leaf = self
                    .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData { contract_id: c.contract_id })
                    .await?;

                let contract_tree_proof = self
                    .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetContractTreeMerkleProof(QSRMerkleCmdGetContractTreeMerkleProof {
                        checkpoint_id: self.get_current_start_checkpoint_id_u64(),
                        contract_id: c.contract_id as u32,
                    }))
                    .await?;

                let result = contract_leaf.to_qfelts();

                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result,
                    witness: DPNStateCmdWitness::ContractLeaf(DPNContractLeafWitness {
                        contract_leaf,
                        contract_tree_proof,
                    }),
                })
            }
            DPNStateCmd::GetCheckpointLeafStats(c) => {
                let requested_checkpoint_id = c.checkpoint_id;
                let checkpoint_leaf_cmd = QSRCmdGetCheckpointLeafData {
                    checkpoint_id: requested_checkpoint_id,
                };
                let checkpoint_leaf = self.resolve_get_checkpoint_leaf_mut(&checkpoint_leaf_cmd).await?;

                let state_roots = self.get_checkpoint_state_roots(requested_checkpoint_id).await?;

                let current_checkpoint_id = self.get_current_start_checkpoint_id_u64();
                let historical_proof = self
                    .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetCheckpointTreeMerkleProof(QSRMerkleCmdGetCheckpointTreeMerkleProof {
                        checkpoint_id: current_checkpoint_id,
                        leaf_checkpoint_id: requested_checkpoint_id,
                    }))
                    .await?;

                let mut result = Vec::new();
                result.extend_from_slice(&checkpoint_leaf.stats.to_qfelts());

                Ok(PsyCmdWithInputAndWitness {
                    state_cmd: state_cmd.clone(),
                    result,
                    witness: DPNStateCmdWitness::CheckpointLeafStats(DPNCheckpointLeafStatsWitness {
                        checkpoint_leaf_stats: checkpoint_leaf.stats,
                        checkpoint_state_roots: state_roots,
                        checkpoint_historical_proof: historical_proof,
                    }),
                })
            }
            DPNStateCmd::ClearEntireTree(c) => {
                let current_contract_id = self.get_current_contract_id();

                let contract_leaf = self
                    .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData {
                        contract_id: current_contract_id.to_canonical_u64(),
                    })
                    .await?;

                let state_tree_height = contract_leaf.state_tree_height.to_canonical_u64();

                if c.condition == 0 {
                    let current_state_root = self.get_contract_state_slot(current_contract_id, F::ZERO).await?.root;

                    return Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: current_state_root
                            .0
                            .elements
                            .iter()
                            .map(|x| F::from_noncanonical_u64(x.to_canonical_u64()))
                            .collect(),
                        witness: DPNStateCmdWitness::ClearEntireTree(DPNClearEntireTreeWitness {
                            state_tree_height,
                            zero_hash: current_state_root,
                        }),
                    });
                } else {
                    let zero_hash_psy = <Self as PsyReadLocalProvingSessionStoreMut<F>>::Hasher::get_zero_hash(state_tree_height as usize);

                    self.notify_clear_entire_tree(current_contract_id.to_canonical_u64()).await?;

                    let zero_hash_felts: Vec<F> = zero_hash_psy
                        .0
                        .elements
                        .iter()
                        .map(|x| F::from_noncanonical_u64(x.to_canonical_u64()))
                        .collect();
                    let zero_hash = QHashOut::from_felt_slice(&zero_hash_felts);

                    Ok(PsyCmdWithInputAndWitness {
                        state_cmd: state_cmd.clone(),
                        result: zero_hash_felts,
                        witness: DPNStateCmdWitness::ClearEntireTree(DPNClearEntireTreeWitness {
                            state_tree_height,
                            zero_hash,
                        }),
                    })
                }
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = PsyFelt))]
pub struct PsyCmdWithInputAndWitness<F: RichField> {
    pub state_cmd: DPNStateCmd<u64>,
    pub witness: DPNStateCmdWitness<F>,
    pub result: Vec<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct PsyEvalSessionResult<F: RichField> {
    pub cmd_witnesses: Vec<PsyCmdWithInputAndWitness<F>>,
}

impl<F: RichField> PsyEvalSessionResult<F> {
    pub fn new() -> Self {
        Self { cmd_witnesses: Vec::new() }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<F: RichField + PrimeField64> PsyEvalSessionResult<F> {
    pub async fn process_state_cmd<S>(&mut self, executor: &mut SimpleDPNExecutor<F>, sesh: &mut S, cmd: &DPNStateCmd<u64>) -> anyhow::Result<()>
    where
        S: PsyReadLocalProvingSessionStore<F>
            + PsyEventsStore<F>
            + PsyReadLocalProvingSessionStoreMut<F>
            + PsyCmdInputWitnessResolver<F, <S as PsyReadLocalProvingSessionStoreMut<F>>::Hasher>,
    {
        let real_inputs = cmd
            .get_inputs()
            .iter()
            .map(|x| executor.resolve_target(*x).to_canonical_u64())
            .collect::<Vec<u64>>();
        let new_cmd = cmd.convert_to_u64(&real_inputs);

        let r = sesh.resolve_vec(&new_cmd).await?;
        self.cmd_witnesses.push(r);
        Ok(())
    }

    pub async fn exec_deferred_contract_call<S>(
        self,
        sesh: &mut S,
        contract_id: F,
        caller_contract_id: F,
        fn_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<F>>
    where
        S: PsyReadLocalProvingSessionStore<F>
            + PsyEventsStore<F>
            + PsyReadLocalProvingSessionStoreMut<F>
            + PsyCmdInputWitnessResolver<F, <S as PsyReadLocalProvingSessionStoreMut<F>>::Hasher>,
    {
        sesh.init_transaction(DPNProvingSessionSimpleMethodCall {
            caller_contract_id,
            contract_id,
            method_id: F::from_canonical_u32(fn_def.method_id),
            inputs: inputs.clone(),
        })
        .await?;
        self.eval_session(fn_def, sesh, inputs).await
    }

    pub async fn exec_contract_call<S>(
        self,
        sesh: &mut S,
        contract_id: F,
        fn_def: &DPNFunctionCircuitDefinition,
        inputs: Vec<F>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<F>>
    where
        S: PsyReadLocalProvingSessionStore<F>
            + PsyEventsStore<F>
            + PsyReadLocalProvingSessionStoreMut<F>
            + PsyCmdInputWitnessResolver<F, <S as PsyReadLocalProvingSessionStoreMut<F>>::Hasher>,
    {
        self.exec_deferred_contract_call(sesh, contract_id, F::from_canonical_u64(DEFAULT_CALLER_CONTRACT_ID_U64), fn_def, inputs)
            .await
    }

    async fn eval_session<S>(
        mut self,
        fn_def: &DPNFunctionCircuitDefinition,
        sesh: &mut S,
        inputs: Vec<F>,
    ) -> anyhow::Result<DapenContractFunctionCircuitInput<F>>
    where
        S: PsyReadLocalProvingSessionStore<F>
            + PsyEventsStore<F>
            + PsyReadLocalProvingSessionStoreMut<F>
            + PsyCmdInputWitnessResolver<F, <S as PsyReadLocalProvingSessionStoreMut<F>>::Hasher>,
    {
        let start_session_ctx = sesh.get_fresh_start_ctx_for_user(sesh.get_current_user_id()).await?;
        let call_data_ctx = sesh
            .get_call_start_data(sesh.get_current_contract_id(), F::from_canonical_u32(fn_def.method_id), &inputs)
            .await?;

        let inputs_clone = inputs.clone();
        let mut executor = SimpleDPNExecutor::<F>::new_with_contract_ctx(
            inputs,
            sesh.get_current_user_id(),
            sesh.get_current_contract_id(),
            sesh.get_current_caller_contract_id(),
            sesh.get_current_start_checkpoint_id(),
            sesh.get_nonce(),
            start_session_ctx.start_session_user_leaf.public_key.0.elements,
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
                self.process_state_cmd(&mut executor, sesh, &fn_def.state_commands[next_state_cmd_id])
                    .await?;
                next_state_cmd_id += 1;
                if next_state_cmd_id >= state_cmd_len {
                    next_state_cmd_index = fn_def.definitions.len() + 10;
                } else {
                    next_state_cmd_index = fn_def.state_command_resolution_indices[next_state_cmd_id];
                }
            }
        }
        for assertion in fn_def.assertions.iter() {
            let left = executor.resolve_target(assertion.left).to_canonical_u64();
            let right = executor.resolve_target(assertion.right).to_canonical_u64();
            if left != right {
                anyhow::bail!("assertion failed: {} (left: {}, right: {})", assertion.message, left, right);
            }
        }

        let mut events = Vec::new();
        let start_event_index = sesh.get_event_index();
        for (i, event) in fn_def.events.iter().enumerate() {
            let event_record = PsyUserEventRecord {
                checkpoint_id: executor.resolve_target(event.checkpoint_id),
                user_id: executor.resolve_target(event.user_id),
                contract_id: executor.resolve_target(event.contract_id),
                method_id: F::from_canonical_u32(fn_def.method_id),
                event_index: start_event_index + F::from_noncanonical_u64(i as u64),
                data: event.data.iter().map(|x| executor.resolve_target(*x)).collect::<Vec<F>>(),
            };
            events.push(event_record);
        }

        let total_events_emitted = F::from_noncanonical_u64(events.len() as u64);
        sesh.write_events(events.clone());

        let outputs = fn_def.circuit_outputs.iter().map(|x| executor.resolve_target(*x)).collect::<Vec<F>>();
        let end_ctx = DapenCFCUserTransactionEndContext {
            end_contract_state_tree_root: sesh.get_contract_state_slot(sesh.get_current_contract_id(), F::ZERO).await?.root,
            end_deferred_tx_debt_tree_root: sesh.get_latest_deferred_tx_leaf()?.root,
            outputs_hash: safe_hash_fixed_length::<<S as PsyReadLocalProvingSessionStoreMut<F>>::Hasher, F>(&outputs),
            outputs_length: F::from_noncanonical_u64(outputs.len() as u64),
            total_events_emitted,
            total_balance_spent: F::from_noncanonical_u64(0),
        };
        let input_ctx = DapenCFCUserTransactionInputContext {
            proving_session_start_ctx: start_session_ctx,
            transaction_call_start_ctx: call_data_ctx,
            transaction_end_ctx: end_ctx,
        };

        sesh.finalize_transaction().await?;

        Ok(DapenContractFunctionCircuitInput {
            inputs: inputs_clone,
            outputs,
            events,
            cmd_witnesses: self.cmd_witnesses,
            session_proof_tree_root: sesh.get_q_recursion_proof_tree_root(),
            tx_input_ctx: input_ctx,
        })
    }
}
