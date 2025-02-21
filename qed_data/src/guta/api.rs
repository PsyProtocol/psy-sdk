use kvq::traits::KVQSerializable;
use plonky2::{
    field::extension::Extendable,
    hash::hash_types::RichField,
    plonk::{config::GenericConfig, proof::ProofWithPublicInputs},
};
use qed_core::{data::qhashout::QHashOut, job::{drain_queue::{DrainQueueMetadata, DrainQueueMetadataTagged}, id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID}}};
use qed_crypto::hash::{merkle::core::{DeltaMerkleProofCore, MerkleProofCore}, traits::{hasher::FieldQHasher, qhashable::QFieldHashable}};
use serde::{Deserialize, Serialize};

use crate::{qdata::{ups_end_cap_result::UPSEndCapResultCompact, user::QEDUserLeaf}, qstore::uct_merkle_nodes::CSTUserUpdateStore};

use super::{end_cap_input::SubmitUserEndCapNonProofInput, proof_input::VerifyEndCapSimpleStandardInput, stats::GUTAStats};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractStateUpdateHistory<F: RichField> {
    pub user_contract_tree_update_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub contract_state_tree_updates: Vec<DeltaMerkleProofCore<QHashOut<F>>>,
}

impl<F: RichField> QEDContractStateUpdateHistory<F> {
    pub fn ensure_basic_consistency(&self) -> anyhow::Result<()> {
        if self.contract_state_tree_updates.len() == 0 {
            anyhow::bail!("contract_state_tree_updates cannot be empty")
        }
        if self.contract_state_tree_updates[0].old_root != self.user_contract_tree_update_proof.old_value {
            anyhow::bail!("first CST old root does not match UCT old value");
        }
        if self.contract_state_tree_updates.last().as_ref().unwrap().new_root != self.user_contract_tree_update_proof.new_value {
            anyhow::bail!("first CST new root does not match UCT new value");
        }


        for i in 1..self.contract_state_tree_updates.len() {
            if self.contract_state_tree_updates[i].old_root != self.contract_state_tree_updates[i-1].new_root {
                anyhow::bail!("invalid cst transition proof: current old_root != last new_root");
            }
        }


       Ok(())

    }
    pub fn verify_generate_cst_delta<H: FieldQHasher<F>>(&self, injestor: &mut CSTUserUpdateStore<QHashOut<F>>) -> anyhow::Result<()> {


        injestor.verify_injest_uct_delta_merkle_proof::<H>(&self.user_contract_tree_update_proof)?;

        let contract_id = self.user_contract_tree_update_proof.index as u32;
        

        for p in self.contract_state_tree_updates.iter() {
            injestor.verify_injest_delta_merkle_proof::<H>(contract_id, p)?;
        }

        Ok(())

        

    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct UserEndCapNonProofCoreInputQueueItem<F: RichField> {
    pub input: SubmitUserEndCapNonProofCoreInput<F>,
    pub checkpoint_tree_proof: MerkleProofCore<QHashOut<F>>,
    pub proof_id: QProvingJobDataID,
    pub channel_id: u64,
}

impl<F: RichField> UserEndCapNonProofCoreInputQueueItem<F> {
    pub fn get_verify_end_cap_simple_input(&self) -> VerifyEndCapSimpleStandardInput<F> {
        VerifyEndCapSimpleStandardInput {
            guta_stats: self.input.stats,
            checkpoint_root: self.input.state_transition.checkpoint_tree_root_hash,
            checkpoint_historical_merkle_proof: self.checkpoint_tree_proof.clone(),
        }

    }
}

impl<F: RichField> DrainQueueMetadataTagged for UserEndCapNonProofCoreInputQueueItem<F> {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {
        DrainQueueMetadata {
            channel_id: self.channel_id,
            checkpoint_id: self.input.checkpoint_id.to_canonical_u64(),
            item_id: self.input.new_user_leaf.user_id.to_canonical_u64(),
        }
    }
}
impl<F: RichField> KVQSerializable for UserEndCapNonProofCoreInputQueueItem<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}




#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitUserEndCapNonProofCoreInput<F: RichField> {
    pub checkpoint_id: F,
    pub stats: GUTAStats<F>,
    pub state_transition: UPSEndCapResultCompact<F>,
    pub new_user_leaf: QEDUserLeaf<F>,
}
impl<F: RichField> SubmitUserEndCapNonProofCoreInput<F> {

    pub fn get_proof_public_inputs_hash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_two_to_one(
            self.state_transition.qfhash::<H>(),
            self.stats.qfhash::<H>()
        )
    }
}

impl<F: RichField> KVQSerializable for SubmitUserEndCapNonProofCoreInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitUserEndCapProofAPIInput<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub input: SubmitUserEndCapNonProofInput<F>,
    pub proof: ProofWithPublicInputs<F, C, D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> KVQSerializable
    for SubmitUserEndCapProofAPIInput<F, C, D>
{
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitUserEndCapProofIDAPIInput<F: RichField> {
    pub input: SubmitUserEndCapNonProofInput<F>,
    pub proof_id: QProvingJobDataID,
}

impl<F: RichField> KVQSerializable for SubmitUserEndCapProofIDAPIInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}







#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitGUTARealmResultAPINoProofInput<F: RichField> {
    pub realm_id: u64,
    pub checkpoint_id: u64,
    pub guta_stats: GUTAStats<F>,
    pub top_line_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub checkpoint_tree_root: QHashOut<F>,
    pub circuit_type: ProvingJobCircuitType,
}
impl<F: RichField> KVQSerializable for SubmitGUTARealmResultAPINoProofInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitGUTARealmResultAPIQueueItem<F: RichField> {
    pub realm_id: u64,
    pub guta_channel_id: u64,
    pub checkpoint_id: u64,
    pub guta_stats: GUTAStats<F>,
    pub top_line_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub checkpoint_tree_root: QHashOut<F>,
    pub proof_id: QProvingJobDataID,
}
impl<F: RichField> KVQSerializable for SubmitGUTARealmResultAPIQueueItem<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> DrainQueueMetadataTagged for SubmitGUTARealmResultAPIQueueItem<F> {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {
        DrainQueueMetadata {
            channel_id: self.guta_channel_id,
            checkpoint_id: self.checkpoint_id,
            item_id: self.realm_id,
        }
    }

}

impl<F: RichField > SubmitGUTARealmResultAPINoProofInput<F> {
    pub fn to_queue_item(self, guta_channel_id: u64, realm_root_level: u32) -> SubmitGUTARealmResultAPIQueueItem<F> {
        SubmitGUTARealmResultAPIQueueItem {
            realm_id: self.realm_id,
            guta_channel_id: guta_channel_id,
            checkpoint_id:self.checkpoint_id,
            guta_stats: self.guta_stats,
            top_line_proof: self.top_line_proof,
            checkpoint_tree_root:self.checkpoint_tree_root,
            proof_id: QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                self.checkpoint_id,
                self.circuit_type.to_circuit_group_id(),
                realm_root_level,
                self.realm_id as u32,
                self.circuit_type,
                ProvingJobDataType::OutputProof,
                0,
            )
        }
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitGUTARealmResultAPIWithProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub input: SubmitGUTARealmResultAPINoProofInput<F>,
    pub proof: ProofWithPublicInputs<F,C,D>
}
impl<
F: RichField + Extendable<D>,
C: GenericConfig<D, F = F>,
const D: usize,
> KVQSerializable for SubmitGUTARealmResultAPIWithProof<F,C,D> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}