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
use qed_common_circuit::{
    builder::{comparison::CircuitBuilderComparison, hash::core::CircuitBuilderHashCore},
    circuits::traits::qstandard::QStandardCircuit,
    hash::merkle::gadgets::merkle_proof::MerkleProofGadget,
    proof_minifier::pm_chain::QEDProofMinifierChain,
    u32::gates::comparison::ComparisonGate,
};
use qed_core::{
    config::network_constants::MAX_CONTRACT_STATE_TREE_HEIGHT, data::qhashout::QHashOut,
};
use qed_crypto::{
    hash::{
        merkle::core::MerkleProofCore,
        traits::hasher::{MerkleHasher, MerkleZeroHasher},
    },
    signature::zk::wallet::PRIVATE_KEY_CONSTANTS,
};
use qed_data::{
    config::store_config::QEDHasher,
    models::user::contract_state_tree::UserContractStateTreeId,
    qdata::user_contract_state::UserContractState,
    qstore::imm::{
        cache::QEDCmdStoreWithCache,
        cmd::{
            QSRCmdGetContractLeafData, QSRMerkleCmd,
            QSRMerkleCmdGetUserContractStateTreeMerkleProof,
        },
        cmd_processor::{QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut},
    },
};
use qed_rollup_circuit::gadgets::qdata::user_contract_state::UserContractStateGadget;
use qedlang_core::dpn::ops::state_cmd::data::{
    DPNStateCmd, DPNStateCmdGetOtherUserContractStateSlotHash,
    DPNStateCmdGetSelfUserCurrentContractStateSlotHash,
    DPNStateCmdGetSelfUserExternalContractStateSlotHash,
};

#[derive(Debug)]
pub struct StateReaderGadget<F: RichField + Extendable<D>, const D: usize> {
    pub state: UserContractStateGadget,
    pub contract_state_tree_height: u8,
    pub merkel_proofs: Vec<MerkleProofGadget>,
    pub state_cmds: Vec<DPNStateCmd<F>>,
    pub current_state_cmd_index: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> StateReaderGadget<F, D> {
    pub fn new(builder: &mut CircuitBuilder<F, D>, contract_state_tree_height: u8) -> Self {
        let state = UserContractStateGadget::add_virtual_to(builder);
        Self {
            state,
            contract_state_tree_height,
            merkel_proofs: Vec::new(),
            state_cmds: Vec::new(),
            current_state_cmd_index: 0,
        }
    }
    pub fn set_witness<R: QEDReadCommandProcessorSync<F> + Send + Sync>(
        &self,
        pw: &mut PartialWitness<F>,
        state_reader: &StateReader<F, D, R>,
    ) -> anyhow::Result<()> {
        assert_eq!(self.state_cmds.len(), self.merkel_proofs.len());
        assert_eq!(
            state_reader.state_cmds.len(),
            state_reader.merkel_proofs.len()
        );
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
            .try_for_each(|(merkle_proof_gadget, merkle_proof)| {
                merkle_proof_gadget.set_witness_core_proof_q_generic(pw, &merkle_proof)
            })?;

        Ok(())
    }
    pub fn get_self_user_current_contract_state_slot_hash(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        slot_index: F,
    ) -> anyhow::Result<HashOutTarget> {
        let merkle_proof_gadget = MerkleProofGadget::add_virtual_to::<PoseidonHash, F, D>(
            builder,
            self.contract_state_tree_height as usize,
        );
        tracing::info!("merkle_proof_gadget.root: {:?}", merkle_proof_gadget.root);
        tracing::info!(
            "self.state.start_contract_state_root: {:?}",
            self.state.start_contract_state_root
        );
        builder.connect_hashes(
            merkle_proof_gadget.root,
            self.state.start_contract_state_root,
        );
        let expected_slot_index_target = builder.constant(slot_index);
        builder.connect(merkle_proof_gadget.index, expected_slot_index_target);

        let value = merkle_proof_gadget.value.clone();

        self.merkel_proofs.push(merkle_proof_gadget);
        self.state_cmds
            .push(DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(
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
            let value_0 =
                self.get_self_user_current_contract_state_slot_hash(builder, slot_index)?;
            let value_1 =
                self.get_self_user_current_contract_state_slot_hash(builder, slot_index + F::ONE)?;

            let elements = [value_0.elements, value_1.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<Target>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self.get_self_user_current_contract_state_slot_hash(
                    builder,
                    F::from_canonical_u64(start_slot + i),
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
                    let slot_mask_type =
                        (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
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
        let merkle_proof_gadget = MerkleProofGadget::add_virtual_to::<PoseidonHash, F, D>(
            builder,
            contract_state_tree_height as usize,
        );

        builder.connect_hashes(
            merkle_proof_gadget.root,
            self.state.start_contract_state_root,
        );
        tracing::info!("merkle_proof_gadget.root: {:?}", merkle_proof_gadget.root);
        tracing::info!(
            "self.state.start_contract_state_root: {:?}",
            self.state.start_contract_state_root
        );
        let expected_slot_index_target = builder.constant(slot_index);
        builder.connect(merkle_proof_gadget.index, expected_slot_index_target);

        let value = merkle_proof_gadget.value.clone();

        self.merkel_proofs.push(merkle_proof_gadget);
        self.state_cmds
            .push(DPNStateCmd::GetSelfUserExternalContractStateSlotHash(
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
        let value = self.get_self_user_external_contract_state_slot_hash(
            builder,
            contract_id,
            slot_index,
            contract_state_tree_height,
        )?;
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
            let cur = self.get_self_user_external_contract_state_slot_hash(
                builder,
                contract_id,
                slot_index,
                contract_state_tree_height,
            )?;
            Ok(vec![cur.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_self_user_external_contract_state_slot_hash(
                builder,
                contract_id,
                slot_index,
                contract_state_tree_height,
            )?;
            let value_1 = self.get_self_user_external_contract_state_slot_hash(
                builder,
                contract_id,
                slot_index + F::ONE,
                contract_state_tree_height,
            )?;

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
                    let slot_mask_type =
                        (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
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
        let merkle_proof_gadget = MerkleProofGadget::add_virtual_to::<PoseidonHash, F, D>(
            builder,
            contract_state_tree_height as usize,
        );
        builder.connect_hashes(
            merkle_proof_gadget.root,
            self.state.start_contract_state_root,
        );
        let expected_slot_index = builder.constant(slot_index);
        builder.connect(merkle_proof_gadget.index, expected_slot_index);

        let value = merkle_proof_gadget.value.clone();

        self.merkel_proofs.push(merkle_proof_gadget);
        self.state_cmds
            .push(DPNStateCmd::GetOtherUserContractStateSlotHash(
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
        let value = self.get_other_user_contract_state_slot_hash(
            builder,
            user_id,
            contract_id,
            slot_index,
            contract_state_tree_height,
        )?;

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
            let cur = self.get_other_user_contract_state_slot_hash(
                builder,
                user_id,
                contract_id,
                slot_index,
                contract_state_tree_height,
            )?;
            Ok(vec![cur.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_other_user_contract_state_slot_hash(
                builder,
                user_id,
                contract_id,
                slot_index,
                contract_state_tree_height,
            )?;
            let value_1 = self.get_other_user_contract_state_slot_hash(
                builder,
                user_id,
                contract_id,
                slot_index + F::ONE,
                contract_state_tree_height,
            )?;

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
                    let slot_mask_type =
                        (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
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

pub struct StateReader<
    F: RichField + Extendable<D>,
    const D: usize,
    R: QEDReadCommandProcessorSync<F> + Send + Sync,
> {
    pub state: UserContractState<F>,
    pub cmd_store: QEDCmdStoreWithCache<F, R>,
    pub state_tree_store: KVQSimpleMemoryBackingStore,
    pub merkel_proofs: Vec<MerkleProofCore<QHashOut<F>>>,
    pub state_cmds: Vec<DPNStateCmd<F>>,
}

impl<
        F: RichField + Extendable<D>,
        const D: usize,
        R: QEDReadCommandProcessorSync<F> + Send + Sync,
    > StateReader<F, D, R>
{
    pub fn new(
        state: UserContractState<F>,
        cmd_store: QEDCmdStoreWithCache<F, R>,
        state_tree_store: KVQSimpleMemoryBackingStore,
    ) -> Self {
        Self {
            state,
            cmd_store,
            state_tree_store,
            merkel_proofs: Vec::new(),
            state_cmds: Vec::new(),
        }
    }

    fn get_user_contract_state_tree_merkle_proof(
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
            .resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData { contract_id })?
            .state_tree_height
            .to_canonical_u64() as u8;
        let id = UserContractStateTreeId::<KVQSimpleMemoryBackingStore>::new(
            user_id,
            contract_id as u32,
            state_tree_height,
        );
        let base_mp = self.cmd_store.resolve_get_merkle_proof_mut(
            &QSRMerkleCmd::GetUserContractStateTreeMerkleProof(
                QSRMerkleCmdGetUserContractStateTreeMerkleProof {
                    checkpoint_id,
                    user_id,
                    contract_id: contract_id as u32,
                    height: state_tree_height,
                    leaf_id: slot_index,
                },
            ),
        )?;
        let base_mp_gf = serde_json::from_str::<MerkleProofCore<QHashOut<GoldilocksField>>>(
            &serde_json::to_string(&base_mp)?,
        )?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, checkpoint_id, &base_mp_gf)?;
        let merkel_proof = id.get_leaf_ucs(&self.state_tree_store, checkpoint_id, slot_index)?;
        let merkel_proof_f = serde_json::from_str::<MerkleProofCore<QHashOut<F>>>(
            &serde_json::to_string(&merkel_proof)?,
        )?;
        Ok(merkel_proof_f)
    }
    pub fn get_self_user_current_contract_state_slot_hash(
        &mut self,
        slot_index: F,
    ) -> anyhow::Result<QHashOut<F>> {
        let merkle_proof = self.get_user_contract_state_tree_merkle_proof(
            self.state.checkpoint_id,
            self.state.user_leaf.user_id,
            self.state.contract_id,
            slot_index,
        )?;
        tracing::info!(
            "merkle_proof: {}",
            serde_json::to_string_pretty(&merkle_proof)?
        );

        let mut current = merkle_proof.value;
        for (i, sibling) in merkle_proof.siblings.iter().enumerate() {
            if merkle_proof.index & (1 << i) == 0 {
                current = QEDHasher::two_to_one(&current, sibling);
            } else {
                current = QEDHasher::two_to_one(sibling, &current);
            }
        }

        tracing::info!("calc root: {}", current.to_string());

        let value = merkle_proof.value.clone();

        self.merkel_proofs.push(merkle_proof);
        self.state_cmds
            .push(DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(
                DPNStateCmdGetSelfUserCurrentContractStateSlotHash { slot_index },
            ));

        Ok(value)
    }

    pub fn get_self_user_current_contract_state_slot_single(
        &mut self,
        sub_slot_index: F,
    ) -> anyhow::Result<F> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value = self.get_self_user_current_contract_state_slot_hash(slot_index)?;
        Ok(value.0.elements[slot_offset as usize])
    }

    pub fn get_self_user_current_contract_state_slot_range(
        &mut self,
        sub_slot_index: F,
        length: u32,
    ) -> anyhow::Result<Vec<F>> {
        let sub_slot_index = sub_slot_index.to_noncanonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let n = (sub_slot_index & 0b11) as usize;
        if length == 1 {
            // one merkle proof
            let cur = self.get_self_user_current_contract_state_slot_hash(slot_index)?;
            Ok(vec![cur.0.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 = self.get_self_user_current_contract_state_slot_hash(slot_index)?;
            let value_1 =
                self.get_self_user_current_contract_state_slot_hash(slot_index + F::ONE)?;

            let elements = [value_0.0.elements, value_1.0.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<F>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self.get_self_user_current_contract_state_slot_hash(
                    F::from_canonical_u64(start_slot + i),
                )?;
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
                    let slot_mask_type =
                        (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
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

    pub fn get_self_user_external_contract_state_slot_hash(
        &mut self,
        contract_id: F,
        slot_index: F,
    ) -> anyhow::Result<QHashOut<F>> {
        let merkle_proof = self.get_user_contract_state_tree_merkle_proof(
            self.state.checkpoint_id,
            self.state.user_leaf.user_id,
            contract_id,
            slot_index,
        )?;

        let value = merkle_proof.value.clone();

        let state_tree_height = merkle_proof.siblings.len() as u8;

        self.merkel_proofs.push(merkle_proof);
        self.state_cmds
            .push(DPNStateCmd::GetSelfUserExternalContractStateSlotHash(
                DPNStateCmdGetSelfUserExternalContractStateSlotHash {
                    contract_id,
                    slot_index,
                    contract_state_tree_height: state_tree_height,
                },
            ));

        Ok(value)
    }

    pub fn get_self_user_external_contract_state_slot_single(
        &mut self,
        contract_id: F,
        sub_slot_index: F,
    ) -> anyhow::Result<F> {
        let sub_slot_index = sub_slot_index.to_canonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value =
            self.get_self_user_external_contract_state_slot_hash(contract_id, slot_index)?;
        Ok(value.0.elements[slot_offset as usize])
    }

    pub fn get_self_user_external_contract_state_slot_range(
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
            let cur =
                self.get_self_user_external_contract_state_slot_hash(contract_id, slot_index)?;
            Ok(vec![cur.0.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 =
                self.get_self_user_external_contract_state_slot_hash(contract_id, slot_index)?;
            let value_1 = self.get_self_user_external_contract_state_slot_hash(
                contract_id,
                slot_index + F::ONE,
            )?;

            let elements = [value_0.0.elements, value_1.0.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<F>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self.get_self_user_external_contract_state_slot_hash(
                    contract_id,
                    F::from_canonical_u64(start_slot + i),
                )?;
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
                    let slot_mask_type =
                        (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
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

    pub fn get_other_user_contract_state_slot_hash(
        &mut self,
        user_id: F,
        contract_id: F,
        slot_index: F,
    ) -> anyhow::Result<QHashOut<F>> {
        let merkle_proof = self.get_user_contract_state_tree_merkle_proof(
            self.state.checkpoint_id,
            user_id,
            contract_id,
            slot_index,
        )?;
        let state_tree_height = merkle_proof.siblings.len() as u8;

        let value = merkle_proof.value.clone();

        self.merkel_proofs.push(merkle_proof);
        self.state_cmds
            .push(DPNStateCmd::GetOtherUserContractStateSlotHash(
                DPNStateCmdGetOtherUserContractStateSlotHash {
                    user_id,
                    contract_id,
                    slot_index,
                    contract_state_tree_height: state_tree_height,
                },
            ));

        Ok(value)
    }

    pub fn get_other_user_contract_state_slot_single(
        &mut self,
        user_id: F,
        contract_id: F,
        sub_slot_index: F,
    ) -> anyhow::Result<F> {
        let sub_slot_index = sub_slot_index.to_canonical_u64();
        let slot_index = F::from_canonical_u64(sub_slot_index / 4u64);
        let slot_offset = sub_slot_index % 4u64;
        let value =
            self.get_other_user_contract_state_slot_hash(user_id, contract_id, slot_index)?;

        Ok(value.0.elements[slot_offset as usize])
    }

    pub fn get_other_user_contract_state_slot_range(
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
            let cur =
                self.get_other_user_contract_state_slot_hash(user_id, contract_id, slot_index)?;
            Ok(vec![cur.0.elements[n]])
        } else if length < 6 {
            // two merkle proofs
            let value_0 =
                self.get_other_user_contract_state_slot_hash(user_id, contract_id, slot_index)?;
            let value_1 = self.get_other_user_contract_state_slot_hash(
                user_id,
                contract_id,
                slot_index + F::ONE,
            )?;

            let elements = [value_0.0.elements, value_1.0.elements].concat();

            Ok(elements[n..(n + length as usize)].to_vec())
        } else {
            let n_proofs = ((length + 6) / 4) as u64;
            let sub_slot_index_mod_4 = sub_slot_index % 4;
            let start_slot = sub_slot_index / 4;
            let mut result = Vec::<F>::with_capacity(length as usize);

            let len_minus_2_mod_4 = (length - 2) % 4;

            for i in 0..n_proofs {
                let mp_value = self.get_other_user_contract_state_slot_hash(
                    user_id,
                    contract_id,
                    F::from_canonical_u64(start_slot + i),
                )?;
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
                    let slot_mask_type =
                        (len_minus_2_mod_4 as usize) + sub_slot_index_mod_4 as usize;
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

pub trait SoftwareDefinedSignTrait {
    fn get_public_key_f<C: GenericConfig<D>, const D: usize>(
        builder: &mut CircuitBuilder<C::F, D>,
        private_key: HashOutTarget,
    ) -> HashOutTarget
    where
        C::Hasher: AlgebraicHasher<C::F>;

    fn get_public_key<F: RichField, H: AlgebraicHasher<F>>(private_key: HashOut<F>) -> HashOut<F>;

    fn custom_sign_option_f<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        state_reader: &mut StateReaderGadget<F, D>,
        sig_inputs: Vec<Target>,
    ) -> anyhow::Result<()>;

    fn custom_sign_option<
        F: RichField + Extendable<D>,
        const D: usize,
        R: QEDReadCommandProcessorSync<F> + Send + Sync,
    >(
        state_reader: &mut StateReader<F, D, R>,
        sig_inputs: Vec<F>,
    ) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct SoftwareDefinedSignGadget {
    pub private_key: HashOutTarget,
}

impl SoftwareDefinedSignTrait for SoftwareDefinedSignGadget {
    fn get_public_key_f<C: GenericConfig<D>, const D: usize>(
        builder: &mut CircuitBuilder<C::F, D>,
        private_key: HashOutTarget,
    ) -> HashOutTarget
    where
        C::Hasher: AlgebraicHasher<C::F>,
    {
        let private_key_constants = PRIVATE_KEY_CONSTANTS
            .iter()
            .map(|c| builder.constant(C::F::from_canonical_u64(*c)))
            .collect::<Vec<_>>();
        builder.hash_n_to_hash_no_pad::<C::Hasher>(vec![
            private_key_constants[0],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[19],
            private_key.elements[1],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[3],
            private_key_constants[4],
            private_key_constants[5],
            private_key_constants[6],
            private_key.elements[0],
            private_key_constants[7],
            private_key.elements[2],
            private_key_constants[8],
            private_key_constants[9],
            private_key_constants[10],
            private_key_constants[11],
            private_key_constants[12],
            private_key.elements[3],
            private_key_constants[13],
            private_key_constants[14],
            private_key_constants[15],
            private_key_constants[16],
            private_key_constants[17],
            private_key_constants[18],
        ])
    }

    fn get_public_key<F: RichField, H: AlgebraicHasher<F>>(private_key: HashOut<F>) -> HashOut<F> {
        let private_key_constants = PRIVATE_KEY_CONSTANTS
            .iter()
            .map(|c| F::from_canonical_u64(*c))
            .collect::<Vec<_>>();
        H::hash_no_pad(&[
            private_key_constants[0],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[19],
            private_key.elements[1],
            private_key_constants[1],
            private_key_constants[2],
            private_key_constants[3],
            private_key_constants[4],
            private_key_constants[5],
            private_key_constants[6],
            private_key.elements[0],
            private_key_constants[7],
            private_key.elements[2],
            private_key_constants[8],
            private_key_constants[9],
            private_key_constants[10],
            private_key_constants[11],
            private_key_constants[12],
            private_key.elements[3],
            private_key_constants[13],
            private_key_constants[14],
            private_key_constants[15],
            private_key_constants[16],
            private_key_constants[17],
            private_key_constants[18],
        ])
    }

    fn custom_sign_option_f<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        state_reader: &mut StateReaderGadget<F, D>,
        sig_inputs: Vec<Target>,
    ) -> anyhow::Result<()> {
        let slot0 = state_reader
            .get_self_user_current_contract_state_slot_single(builder, F::from_canonical_u64(0))?;
        let one_thousand = builder.constant(F::from_canonical_u64(1000));
        builder.ensure_is_less_than(32, slot0, one_thousand);
        Ok(())
    }

    fn custom_sign_option<
        F: RichField + Extendable<D>,
        const D: usize,
        R: QEDReadCommandProcessorSync<F> + Send + Sync,
    >(
        state_reader: &mut StateReader<F, D, R>,
        sig_inputs: Vec<F>,
    ) -> anyhow::Result<()> {
        let slot0 = state_reader
            .get_self_user_current_contract_state_slot_single(F::from_canonical_u64(0))?;
        tracing::info!("slot0: {}", slot0.to_canonical_u64());
        assert!(slot0.to_canonical_u64() < 1000);
        Ok(())
    }
}

#[derive(Debug)]
pub struct SimpleSoftwareDefinedCircuit<
    C: GenericConfig<D>,
    const D: usize,
    S: SoftwareDefinedSignTrait,
> where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub minifier_chain: QEDProofMinifierChain<D, C::F, C>,
    pub circuit_data: CircuitData<C::F, C, D>,
    pub private_key: HashOutTarget,
    pub public_key_param: HashOutTarget,
    pub sig_hash: HashOutTarget,

    pub state_reader: StateReaderGadget<C::F, D>,
    _marker: std::marker::PhantomData<S>,
}

impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignTrait> Clone
    for SimpleSoftwareDefinedCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignTrait>
    SimpleSoftwareDefinedCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_zk_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let private_key = builder.add_virtual_hash();

        let mut state_reader = StateReaderGadget::new(&mut builder, MAX_CONTRACT_STATE_TREE_HEIGHT);

        S::custom_sign_option_f(&mut builder, &mut state_reader, vec![])
            .expect("exec custom sign code failed");

        // gadget
        let public_key_param = S::get_public_key_f::<C, D>(&mut builder, private_key);

        let sig_hash = builder.add_virtual_hash();
        let public_inputs_hash = builder.hash_two_to_one::<C::Hasher>(sig_hash, public_key_param);
        builder.register_public_inputs(&public_inputs_hash.elements);
        let circuit_data = builder.build::<C>();

        let added_gates_for_minifier = [GateRef::new(ComparisonGate::new(32, 16))];

        let minifier_chain = QEDProofMinifierChain::<D, C::F, C>::new_add_gates(
            &circuit_data.verifier_only,
            &circuit_data.common,
            2,
            Some(&added_gates_for_minifier),
        );

        Self {
            circuit_data,
            sig_hash,
            private_key,
            public_key_param,
            state_reader,
            minifier_chain,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn prove_base<R: QEDReadCommandProcessorSync<C::F> + Send + Sync>(
        &self,
        state_reader: &mut StateReader<C::F, D, R>,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::new();
        S::custom_sign_option(state_reader, vec![])?;
        pw.set_hash_target(self.private_key, private_key.0)?;
        pw.set_hash_target(self.sig_hash, sig_hash.0)?;
        self.state_reader.set_witness(&mut pw, state_reader)?;
        let inner_proof = self.circuit_data.prove(pw)?;
        self.minifier_chain.prove(&inner_proof)
    }
}

impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignTrait> QStandardCircuit<C, D>
    for SimpleSoftwareDefinedCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        QHashOut(self.minifier_chain.get_fingerprint())
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        self.minifier_chain.get_verifier_data()
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        self.minifier_chain.get_common_data()
    }
}
