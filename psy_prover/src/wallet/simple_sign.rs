use std::fmt::Debug;

use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use plonky2::{
    field::{
        extension::Extendable,
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    gates::gate::GateRef,
    hash::{
        hash_types::{HashOut, HashOutTarget, RichField},
        poseidon::PoseidonHash,
    },
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common::data::qhashout::QHashOut;
use psy_common_circuit::{
    builder::{comparison::CircuitBuilderComparison, hash::core::CircuitBuilderHashCore},
    circuits::traits::qstandard::QStandardCircuit,
    hash::merkle::gadgets::merkle_proof::MerkleProofGadget,
    proof_minifier::pm_chain::PsyProofMinifierChain,
    u32::gates::comparison::ComparisonGate,
};
use psy_config::network_constants::MAX_CONTRACT_STATE_TREE_HEIGHT;
use psy_crypto::{
    hash::{
        merkle::core::MerkleProofCore,
        traits::hasher::{MerkleHasher, MerkleZeroHasher},
    },
    signature::zk::wallet::PRIVATE_KEY_CONSTANTS,
};
use psy_data::{
    config::store_config::PsyHasher,
    models::user::contract_state_tree::UserContractStateTreeId,
    qdata::user_contract_state::UserContractState,
    qstore::imm::{
        cache::PsyCmdStoreWithCache,
        cmd::{QSRCmdGetContractLeafData, QSRMerkleCmd, QSRMerkleCmdGetUserContractStateTreeMerkleProof},
        cmd_processor::{PsyReadCommandProcessorSync, PsyReadCommandProcessorSyncMut},
    },
};
use psy_network_circuit::gadgets::qdata::user_contract_state::UserContractStateGadget;
use psy_provider::provider::RpcProvider;
use psy_vm::dpn::ops::state_cmd::data::{
    DPNStateCmd, DPNStateCmdGetOtherUserContractStateSlotHash, DPNStateCmdGetSelfUserCurrentContractStateSlotHash,
    DPNStateCmdGetSelfUserExternalContractStateSlotHash,
};

#[derive(Debug)]
pub struct StateReaderGadget<F: RichField + Extendable<D>, const D: usize> {
    pub state: UserContractStateGadget,
    pub contract_state_tree_height: u8,
    pub merkel_proofs: Vec<MerkleProofGadget>,
    pub state_cmds: Vec<DPNStateCmd<F>>,
    // pub current_state_cmd_index: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> StateReaderGadget<F, D> {
    pub fn new(builder: &mut CircuitBuilder<F, D>, contract_state_tree_height: u8) -> Self {
        let state = UserContractStateGadget::add_virtual_to(builder);
        Self {
            state,
            contract_state_tree_height,
            merkel_proofs: Vec::new(),
            state_cmds: Vec::new(),
            // current_state_cmd_index: 0,
        }
    }
    pub fn set_witness<R: PsyReadCommandProcessorSync<F> + Send + Sync>(
        &self,
        pw: &mut PartialWitness<F>,
        state_reader: &StateReader<F, D, R>,
    ) -> anyhow::Result<()> {
        assert_eq!(self.state_cmds.len(), self.merkel_proofs.len());
        assert_eq!(state_reader.state_cmds.len(), state_reader.merkel_proofs.len());
        assert_eq!(self.merkel_proofs.len(), state_reader.merkel_proofs.len());

        self.state.set_witness(pw, &state_reader.state)?;

        self.state_cmds
            .iter()
            .zip(state_reader.state_cmds.iter())
            .for_each(|(state_cmd, state_cmd_reader)| {
                assert_eq!(state_cmd, state_cmd_reader);
            });

        self.merkel_proofs
            .iter()
            .zip(state_reader.merkel_proofs.iter())
            .try_for_each(|(merkle_proof_gadget, merkle_proof)| merkle_proof_gadget.set_witness_core_proof_q_generic(pw, &merkle_proof))?;

        Ok(())
    }
    pub fn get_self_user_current_contract_state_slot_hash(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        slot_index: F,
    ) -> anyhow::Result<HashOutTarget> {
        let merkle_proof_gadget = MerkleProofGadget::add_virtual_to::<PoseidonHash, F, D>(builder, self.contract_state_tree_height as usize);
        builder.connect_hashes(merkle_proof_gadget.root, self.state.start_contract_state_root);
        let expected_slot_index_target = builder.constant(slot_index);
        builder.connect(merkle_proof_gadget.index, expected_slot_index_target);

        let value = merkle_proof_gadget.value.clone();

        self.merkel_proofs.push(merkle_proof_gadget);
        self.state_cmds.push(DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(
            DPNStateCmdGetSelfUserCurrentContractStateSlotHash { slot_index },
        ));

        Ok(value)
    }

    pub fn get_self_user_current_contract_state_slot_single(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        sub_slot_index: F,
    ) -> anyhow::Result<Target> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value = self.get_self_user_current_contract_state_slot_hash(builder, slot_index)?;
        Ok(value.elements[slot_offset as usize])
    }

    pub fn get_self_user_current_contract_state_slot_range(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        sub_slot_index: F,
        length: u32,
    ) -> anyhow::Result<Vec<Target>> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let n = (sub_slot_index & 0b11) as usize;
        if length == 1 {
            // one merkle proof
            let cur = self.get_self_user_current_contract_state_slot_hash(builder, slot_index)?;
            Ok(vec![cur.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_self_user_current_contract_state_slot_hash(builder, slot_index)?;
            let value_1 = self.get_self_user_current_contract_state_slot_hash(builder, slot_index + F::ONE)?;

            let elements = [value_0.elements, value_1.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<Target>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self.get_self_user_current_contract_state_slot_hash(builder, F::from_canonical_u64(start_slot + i))?;
                if i == 0 {
                    if sub_slot_index_mod_4 == 0 {
                        result.push(mp_value.elements[0]);
                        result.push(mp_value.elements[1]);
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 1 {
                        result.push(mp_value.elements[1]);
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 2 {
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 3 {
                        result.push(mp_value.elements[3]);
                    }
                } else if i == (n_proofs - 1) {
                    let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                    if slot_mask_type >= 3 {
                        result.push(mp_value.elements[0]);
                    }
                    if slot_mask_type >= 4 {
                        result.push(mp_value.elements[1]);
                    }
                    if slot_mask_type >= 5 {
                        result.push(mp_value.elements[2]);
                    }
                    if slot_mask_type >= 6 {
                        result.push(mp_value.elements[3]);
                    }
                } else {
                    result.extend_from_slice(&mp_value.elements);
                }
            }
            Ok(result)
        }
    }

    pub fn get_self_user_external_contract_state_slot_hash(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        contract_id: F,
        slot_index: F,
        contract_state_tree_height: u8,
    ) -> anyhow::Result<HashOutTarget> {
        let merkle_proof_gadget = MerkleProofGadget::add_virtual_to::<PoseidonHash, F, D>(builder, contract_state_tree_height as usize);

        builder.connect_hashes(merkle_proof_gadget.root, self.state.start_contract_state_root);
        tracing::info!("merkle_proof_gadget.root: {:?}", merkle_proof_gadget.root);
        tracing::info!("self.state.start_contract_state_root: {:?}", self.state.start_contract_state_root);
        let expected_slot_index_target = builder.constant(slot_index);
        builder.connect(merkle_proof_gadget.index, expected_slot_index_target);

        let value = merkle_proof_gadget.value.clone();

        self.merkel_proofs.push(merkle_proof_gadget);
        self.state_cmds.push(DPNStateCmd::GetSelfUserExternalContractStateSlotHash(
            DPNStateCmdGetSelfUserExternalContractStateSlotHash {
                contract_id,
                slot_index,
                contract_state_tree_height,
            },
        ));

        Ok(value)
    }

    pub fn get_self_user_external_contract_state_slot_single(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        contract_id: F,
        sub_slot_index: F,
        contract_state_tree_height: u8,
    ) -> anyhow::Result<Target> {
        let sub_slot_index = sub_slot_index.to_canonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value = self.get_self_user_external_contract_state_slot_hash(builder, contract_id, slot_index, contract_state_tree_height)?;
        Ok(value.elements[slot_offset as usize])
    }

    pub fn get_self_user_external_contract_state_slot_range(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        contract_id: F,
        sub_slot_index: F,
        length: u32,
        contract_state_tree_height: u8,
    ) -> anyhow::Result<Vec<Target>> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let n = (sub_slot_index & 0b11) as usize;
        if length == 1 {
            // one merkle proof
            let cur = self.get_self_user_external_contract_state_slot_hash(builder, contract_id, slot_index, contract_state_tree_height)?;
            Ok(vec![cur.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_self_user_external_contract_state_slot_hash(builder, contract_id, slot_index, contract_state_tree_height)?;
            let value_1 =
                self.get_self_user_external_contract_state_slot_hash(builder, contract_id, slot_index + F::ONE, contract_state_tree_height)?;

            let elements = [value_0.elements, value_1.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<Target>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self.get_self_user_external_contract_state_slot_hash(
                    builder,
                    contract_id,
                    F::from_canonical_u64(start_slot + i),
                    contract_state_tree_height,
                )?;
                if i == 0 {
                    if sub_slot_index_mod_4 == 0 {
                        result.push(mp_value.elements[0]);
                        result.push(mp_value.elements[1]);
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 1 {
                        result.push(mp_value.elements[1]);
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 2 {
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 3 {
                        result.push(mp_value.elements[3]);
                    }
                } else if i == (n_proofs - 1) {
                    let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                    if slot_mask_type >= 3 {
                        result.push(mp_value.elements[0]);
                    }
                    if slot_mask_type >= 4 {
                        result.push(mp_value.elements[1]);
                    }
                    if slot_mask_type >= 5 {
                        result.push(mp_value.elements[2]);
                    }
                    if slot_mask_type >= 6 {
                        result.push(mp_value.elements[3]);
                    }
                } else {
                    result.extend_from_slice(&mp_value.elements);
                }
            }
            Ok(result)
        }
    }

    pub fn get_other_user_contract_state_slot_hash(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        user_id: F,
        contract_id: F,
        slot_index: F,
        contract_state_tree_height: u8,
    ) -> anyhow::Result<HashOutTarget> {
        let merkle_proof_gadget = MerkleProofGadget::add_virtual_to::<PoseidonHash, F, D>(builder, contract_state_tree_height as usize);
        builder.connect_hashes(merkle_proof_gadget.root, self.state.start_contract_state_root);
        let expected_slot_index = builder.constant(slot_index);
        builder.connect(merkle_proof_gadget.index, expected_slot_index);

        let value = merkle_proof_gadget.value.clone();

        self.merkel_proofs.push(merkle_proof_gadget);
        self.state_cmds.push(DPNStateCmd::GetOtherUserContractStateSlotHash(
            DPNStateCmdGetOtherUserContractStateSlotHash {
                user_id,
                contract_id,
                slot_index,
                contract_state_tree_height,
            },
        ));

        Ok(value)
    }

    pub fn get_other_user_contract_state_slot_single(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        user_id: F,
        contract_id: F,
        sub_slot_index: F,
        contract_state_tree_height: u8,
    ) -> anyhow::Result<Target> {
        let sub_slot_index = sub_slot_index.to_canonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value = self.get_other_user_contract_state_slot_hash(builder, user_id, contract_id, slot_index, contract_state_tree_height)?;

        Ok(value.elements[slot_offset as usize])
    }

    pub fn get_other_user_contract_state_slot_range(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        user_id: F,
        contract_id: F,
        sub_slot_index: F,
        length: u32,
        contract_state_tree_height: u8,
    ) -> anyhow::Result<Vec<Target>> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let n = (sub_slot_index & 0b11) as usize;
        if length == 1 {
            // one merkle proof
            let cur = self.get_other_user_contract_state_slot_hash(builder, user_id, contract_id, slot_index, contract_state_tree_height)?;
            Ok(vec![cur.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_other_user_contract_state_slot_hash(builder, user_id, contract_id, slot_index, contract_state_tree_height)?;
            let value_1 =
                self.get_other_user_contract_state_slot_hash(builder, user_id, contract_id, slot_index + F::ONE, contract_state_tree_height)?;

            let elements = [value_0.elements, value_1.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<Target>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self.get_other_user_contract_state_slot_hash(
                    builder,
                    user_id,
                    contract_id,
                    F::from_canonical_u64(start_slot + i),
                    contract_state_tree_height,
                )?;
                if i == 0 {
                    if sub_slot_index_mod_4 == 0 {
                        result.push(mp_value.elements[0]);
                        result.push(mp_value.elements[1]);
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 1 {
                        result.push(mp_value.elements[1]);
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 2 {
                        result.push(mp_value.elements[2]);
                        result.push(mp_value.elements[3]);
                    } else if sub_slot_index_mod_4 == 3 {
                        result.push(mp_value.elements[3]);
                    }
                } else if i == (n_proofs - 1) {
                    let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                    if slot_mask_type >= 3 {
                        result.push(mp_value.elements[0]);
                    }
                    if slot_mask_type >= 4 {
                        result.push(mp_value.elements[1]);
                    }
                    if slot_mask_type >= 5 {
                        result.push(mp_value.elements[2]);
                    }
                    if slot_mask_type >= 6 {
                        result.push(mp_value.elements[3]);
                    }
                } else {
                    result.extend_from_slice(&mp_value.elements);
                }
            }
            Ok(result)
        }
    }
}

#[derive(Debug, Clone)]
pub struct StateReader<F: RichField + Extendable<D>, const D: usize, R: PsyReadCommandProcessorSync<F> + Send + Sync> {
    pub state: UserContractState<F>,
    pub cmd_store: PsyCmdStoreWithCache<F, R>,
    pub state_tree_store: KVQSimpleMemoryBackingStore,
    pub merkel_proofs: Vec<MerkleProofCore<QHashOut<F>>>,
    pub state_cmds: Vec<DPNStateCmd<F>>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<F: RichField + Extendable<D>, const D: usize, R: PsyReadCommandProcessorSync<F> + Send + Sync> StateReader<F, D, R> {
    pub async fn new(state: UserContractState<F>, cmd_store: PsyCmdStoreWithCache<F, R>, state_tree_store: KVQSimpleMemoryBackingStore) -> Self {
        Self {
            state,
            cmd_store,
            state_tree_store,
            merkel_proofs: Vec::new(),
            state_cmds: Vec::new(),
        }
    }

    async fn get_user_contract_state_tree_merkle_proof(
        &mut self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        slot_index: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        let checkpoint_id = checkpoint_id.to_canonical_u64();
        let user_id = user_id.to_canonical_u64();
        let contract_id = contract_id.to_canonical_u64();
        let slot_index = slot_index.to_canonical_u64();

        let state_tree_height = self
            .cmd_store
            .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData { contract_id })
            .await?
            .state_tree_height
            .to_canonical_u64() as u8;
        let id = UserContractStateTreeId::<KVQSimpleMemoryBackingStore>::new(user_id, contract_id as u32, state_tree_height);
        let base_mp = self
            .cmd_store
            .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                    checkpoint_id,
                    user_id,
                    contract_id: contract_id as u32,
                    height: state_tree_height,
                    leaf_id: slot_index,
                },
            ))
            .await?;
        let base_mp_gf = serde_json::from_str::<MerkleProofCore<QHashOut<GF>>>(&serde_json::to_string(&base_mp)?)?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, checkpoint_id, &base_mp_gf)?;
        let merkel_proof = id.get_leaf_ucs(&self.state_tree_store, checkpoint_id, slot_index)?;
        let merkel_proof_f = serde_json::from_str::<MerkleProofCore<QHashOut<F>>>(&serde_json::to_string(&merkel_proof)?)?;
        Ok(merkel_proof_f)
    }
    pub async fn get_self_user_current_contract_state_slot_hash(&mut self, slot_index: F) -> anyhow::Result<QHashOut<F>> {
        let merkle_proof = self
            .get_user_contract_state_tree_merkle_proof(self.state.checkpoint_id, self.state.user_leaf.user_id, self.state.contract_id, slot_index)
            .await?;
        tracing::info!("merkle_proof: {}", serde_json::to_string_pretty(&merkle_proof)?);

        let value = merkle_proof.value.clone();

        self.merkel_proofs.push(merkle_proof);
        self.state_cmds.push(DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(
            DPNStateCmdGetSelfUserCurrentContractStateSlotHash { slot_index },
        ));

        Ok(value)
    }

    pub async fn get_self_user_current_contract_state_slot_single(&mut self, sub_slot_index: F) -> anyhow::Result<F> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value = self.get_self_user_current_contract_state_slot_hash(slot_index).await?;
        Ok(value.0.elements[slot_offset as usize])
    }

    pub async fn get_self_user_current_contract_state_slot_range(&mut self, sub_slot_index: F, length: u32) -> anyhow::Result<Vec<F>> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let n = (sub_slot_index & 0b11) as usize;
        if length == 1 {
            // one merkle proof
            let cur = self.get_self_user_current_contract_state_slot_hash(slot_index).await?;
            Ok(vec![cur.0.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_self_user_current_contract_state_slot_hash(slot_index).await?;
            let value_1 = self.get_self_user_current_contract_state_slot_hash(slot_index + F::ONE).await?;

            let elements = [value_0.0.elements, value_1.0.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<F>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self
                    .get_self_user_current_contract_state_slot_hash(F::from_canonical_u64(start_slot + i))
                    .await?;
                if i == 0 {
                    if sub_slot_index_mod_4 == 0 {
                        result.push(mp_value.0.elements[0]);
                        result.push(mp_value.0.elements[1]);
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 1 {
                        result.push(mp_value.0.elements[1]);
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 2 {
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 3 {
                        result.push(mp_value.0.elements[3]);
                    }
                } else if i == (n_proofs - 1) {
                    let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                    if slot_mask_type >= 3 {
                        result.push(mp_value.0.elements[0]);
                    }
                    if slot_mask_type >= 4 {
                        result.push(mp_value.0.elements[1]);
                    }
                    if slot_mask_type >= 5 {
                        result.push(mp_value.0.elements[2]);
                    }
                    if slot_mask_type >= 6 {
                        result.push(mp_value.0.elements[3]);
                    }
                } else {
                    result.extend_from_slice(&mp_value.0.elements);
                }
            }
            Ok(result)
        }
    }

    pub async fn get_self_user_external_contract_state_slot_hash(&mut self, contract_id: F, slot_index: F) -> anyhow::Result<QHashOut<F>> {
        let merkle_proof = self
            .get_user_contract_state_tree_merkle_proof(self.state.checkpoint_id, self.state.user_leaf.user_id, contract_id, slot_index)
            .await?;

        let value = merkle_proof.value.clone();

        let state_tree_height = merkle_proof.siblings.len() as u8;

        self.merkel_proofs.push(merkle_proof);
        self.state_cmds.push(DPNStateCmd::GetSelfUserExternalContractStateSlotHash(
            DPNStateCmdGetSelfUserExternalContractStateSlotHash {
                contract_id,
                slot_index,
                contract_state_tree_height: state_tree_height,
            },
        ));

        Ok(value)
    }

    pub async fn get_self_user_external_contract_state_slot_single(&mut self, contract_id: F, sub_slot_index: F) -> anyhow::Result<F> {
        let sub_slot_index = sub_slot_index.to_canonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value = self.get_self_user_external_contract_state_slot_hash(contract_id, slot_index).await?;
        Ok(value.0.elements[slot_offset as usize])
    }

    pub async fn get_self_user_external_contract_state_slot_range(
        &mut self,
        contract_id: F,
        sub_slot_index: F,
        length: u32,
    ) -> anyhow::Result<Vec<F>> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let n = (sub_slot_index & 0b11) as usize;
        if length == 1 {
            // one merkle proof
            let cur = self.get_self_user_external_contract_state_slot_hash(contract_id, slot_index).await?;
            Ok(vec![cur.0.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_self_user_external_contract_state_slot_hash(contract_id, slot_index).await?;
            let value_1 = self
                .get_self_user_external_contract_state_slot_hash(contract_id, slot_index + F::ONE)
                .await?;

            let elements = [value_0.0.elements, value_1.0.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<F>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self
                    .get_self_user_external_contract_state_slot_hash(contract_id, F::from_canonical_u64(start_slot + i))
                    .await?;
                if i == 0 {
                    if sub_slot_index_mod_4 == 0 {
                        result.push(mp_value.0.elements[0]);
                        result.push(mp_value.0.elements[1]);
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 1 {
                        result.push(mp_value.0.elements[1]);
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 2 {
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 3 {
                        result.push(mp_value.0.elements[3]);
                    }
                } else if i == (n_proofs - 1) {
                    let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                    if slot_mask_type >= 3 {
                        result.push(mp_value.0.elements[0]);
                    }
                    if slot_mask_type >= 4 {
                        result.push(mp_value.0.elements[1]);
                    }
                    if slot_mask_type >= 5 {
                        result.push(mp_value.0.elements[2]);
                    }
                    if slot_mask_type >= 6 {
                        result.push(mp_value.0.elements[3]);
                    }
                } else {
                    result.extend_from_slice(&mp_value.0.elements);
                }
            }
            Ok(result)
        }
    }

    pub async fn get_other_user_contract_state_slot_hash(&mut self, user_id: F, contract_id: F, slot_index: F) -> anyhow::Result<QHashOut<F>> {
        let merkle_proof = self
            .get_user_contract_state_tree_merkle_proof(self.state.checkpoint_id, user_id, contract_id, slot_index)
            .await?;
        let state_tree_height = merkle_proof.siblings.len() as u8;

        let value = merkle_proof.value.clone();

        self.merkel_proofs.push(merkle_proof);
        self.state_cmds.push(DPNStateCmd::GetOtherUserContractStateSlotHash(
            DPNStateCmdGetOtherUserContractStateSlotHash {
                user_id,
                contract_id,
                slot_index,
                contract_state_tree_height: state_tree_height,
            },
        ));

        Ok(value)
    }

    pub async fn get_other_user_contract_state_slot_single(&mut self, user_id: F, contract_id: F, sub_slot_index: F) -> anyhow::Result<F> {
        let sub_slot_index = sub_slot_index.to_canonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value = self.get_other_user_contract_state_slot_hash(user_id, contract_id, slot_index).await?;

        Ok(value.0.elements[slot_offset as usize])
    }

    pub async fn get_other_user_contract_state_slot_range(
        &mut self,
        user_id: F,
        contract_id: F,
        sub_slot_index: F,
        length: u32,
    ) -> anyhow::Result<Vec<F>> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let n = (sub_slot_index & 0b11) as usize;
        if length == 1 {
            // one merkle proof
            let cur = self.get_other_user_contract_state_slot_hash(user_id, contract_id, slot_index).await?;
            Ok(vec![cur.0.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_other_user_contract_state_slot_hash(user_id, contract_id, slot_index).await?;
            let value_1 = self
                .get_other_user_contract_state_slot_hash(user_id, contract_id, slot_index + F::ONE)
                .await?;

            let elements = [value_0.0.elements, value_1.0.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<F>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self
                    .get_other_user_contract_state_slot_hash(user_id, contract_id, F::from_canonical_u64(start_slot + i))
                    .await?;
                if i == 0 {
                    if sub_slot_index_mod_4 == 0 {
                        result.push(mp_value.0.elements[0]);
                        result.push(mp_value.0.elements[1]);
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 1 {
                        result.push(mp_value.0.elements[1]);
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 2 {
                        result.push(mp_value.0.elements[2]);
                        result.push(mp_value.0.elements[3]);
                    } else if sub_slot_index_mod_4 == 3 {
                        result.push(mp_value.0.elements[3]);
                    }
                } else if i == (n_proofs - 1) {
                    let slot_mask_type = (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
                    if slot_mask_type >= 3 {
                        result.push(mp_value.0.elements[0]);
                    }
                    if slot_mask_type >= 4 {
                        result.push(mp_value.0.elements[1]);
                    }
                    if slot_mask_type >= 5 {
                        result.push(mp_value.0.elements[2]);
                    }
                    if slot_mask_type >= 6 {
                        result.push(mp_value.0.elements[3]);
                    }
                } else {
                    result.extend_from_slice(&mp_value.0.elements);
                }
            }
            Ok(result)
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub trait SoftwareDefinedSignTrait: Debug + Send + Sync {
    async fn custom_sign_option_f(
        &mut self,
        builder: &mut CircuitBuilder<GF, 2>,
        state_reader: &mut StateReaderGadget<GF, 2>,
        sig_inputs: Vec<Target>,
    ) -> anyhow::Result<()>;

    async fn custom_sign_option(&mut self, state_reader: &mut StateReader<GF, 2, RpcProvider>, sig_inputs: Vec<GF>) -> anyhow::Result<()>;

    fn box_clone(&self) -> Box<dyn SoftwareDefinedSignTrait>;
}

impl Clone for Box<dyn SoftwareDefinedSignTrait> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

type GF = GoldilocksField;
#[derive(Debug, Clone)]
pub struct SoftwareDefinedSignGadget {}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl SoftwareDefinedSignTrait for SoftwareDefinedSignGadget {
    async fn custom_sign_option_f(
        &mut self,
        builder: &mut CircuitBuilder<GF, 2>,
        state_reader: &mut StateReaderGadget<GF, 2>,
        sig_inputs: Vec<Target>,
    ) -> anyhow::Result<()> {
        let slot0 = state_reader.get_self_user_current_contract_state_slot_single(builder, GF::from_canonical_u64(0))?;
        let one_thousand = builder.constant(GF::from_canonical_u64(1000));
        builder.ensure_is_less_than(32, slot0, one_thousand);
        Ok(())
    }

    async fn custom_sign_option(&mut self, state_reader: &mut StateReader<GF, 2, RpcProvider>, sig_inputs: Vec<GF>) -> anyhow::Result<()> {
        let slot0 = state_reader
            .get_self_user_current_contract_state_slot_single(GF::from_canonical_u64(0))
            .await?;
        tracing::info!("slot0: {}", slot0.to_canonical_u64());
        assert!(slot0.to_canonical_u64() < 1000);
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn SoftwareDefinedSignTrait> {
        Box::new(self.clone())
    }
}
