use kvq::traits::KVQSerializable;
use plonky2::{
    field::extension::Extendable,
    hash::hash_types::RichField,
    plonk::{config::GenericConfig, proof::ProofWithPublicInputs},
};
use qed_core::{data::qhashout::QHashOut, job::id::QProvingJobDataID};
use qed_crypto::hash::merkle::core::DeltaMerkleProofCore;
use serde::{Deserialize, Serialize};

use crate::qdata::ups_end_cap_result::UPSEndCapResultCompact;

use super::stats::GUTAStats;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractStateUpdateHistory<F: RichField> {
    pub user_contract_tree_update_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub contract_state_tree_updates: Vec<DeltaMerkleProofCore<QHashOut<F>>>,
}

impl<F: RichField> QEDContractStateUpdateHistory<F> {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitUserEndCapNonProofCoreInput<F: RichField> {
    pub checkpoint_id: F,
    pub stats: GUTAStats<F>,
    pub state_transition: UPSEndCapResultCompact<F>,
}

impl<F: RichField> KVQSerializable for SubmitUserEndCapNonProofCoreInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SubmitUserEndCapNonProofInput<F: RichField> {
    pub core: SubmitUserEndCapNonProofCoreInput<F>,
    pub contract_state_updates: Vec<QEDContractStateUpdateHistory<F>>,
}

impl<F: RichField> KVQSerializable for SubmitUserEndCapNonProofInput<F> {
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



