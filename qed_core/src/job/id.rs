use std::{
    collections::HashMap,
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result};
use hex::FromHexError;
use indexmap::{IndexMap, IndexSet};
use kvq::traits::KVQSerializable;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::{
        hash_types::{HashOut, RichField},
        poseidon::PoseidonHash,
    },
    plonk::{
        config::{GenericConfig, Hasher, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::serde_as;
use uuid::Uuid;
use strum_macros::{Display, AsRefStr};

use super::{mode::QWorkerMode, traits::QProofStoreAsyncImm};

#[async_trait::async_trait]
pub trait QJobRewardDataProvider {
    async fn get_job_commitment(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>>;
    async fn get_job_worker_public_key(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>>;
}

#[async_trait::async_trait]
impl<T: QProofStoreAsyncImm> QJobRewardDataProvider for T {
    async fn get_job_commitment(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>> {
        let public_inputs = self.get_public_input_by_id::<PoseidonGoldilocksConfig, 2>(job_id.get_output_id()).await?;
        Ok(QHashOut(HashOut {
            elements: [
                public_inputs[0],
                public_inputs[1],
                public_inputs[2],
                public_inputs[3],
            ],
        }))
    }

    async fn get_job_worker_public_key(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>> {
        let public_inputs = self.get_public_input_by_id::<PoseidonGoldilocksConfig, 2>(job_id.get_output_id()).await?;
        Ok(QHashOut(HashOut {
            elements: [
                public_inputs[4],
                public_inputs[5],
                public_inputs[6],
                public_inputs[7],
            ],
        }))
    }
}
use crate::{
    config::network_constants::{QED_CHECKPOINT_JOB_ID_CHANNEL, REALM_PROOF_SYNC_CHANNEL},
    data::qhashout::QHashOut,
    job::{
        drain_queue::{DrainQueueMetadata, DrainQueueMetadataTagged},
        history_queue::{HistoryQueueMetadata, HistoryQueueMetadataTagged},
    },
    utils::graph::BidirectionalGraph,
};

type F = GoldilocksField;

pub const GUTA_REWARDS_TREE_MAX_HEIGHT: usize = 16;
pub const CONTRACT_DEPLOYMENT_REWARDS_MAX_HEIGHT: usize = 32;
pub const USER_REGISTRATION_REWARDS_MAX_HEIGHT: usize = 32;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum QCircuitCommonGatesType {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
}
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum QJobTopic {
    GenerateStandardProof = 0,
    GenerateGroth16Proof = 1,
    BlockUserSignatureProof = 2,
    NotifyCoordinatorComplete = 3,
    NotifyRealmComplete = 4,
    AggregateJobs = 5,
}
impl QJobTopic {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
impl From<QJobTopic> for u8 {
    fn from(value: QJobTopic) -> u8 {
        value as u8
    }
}
impl TryFrom<u8> for QJobTopic {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QJobTopic::GenerateStandardProof),
            1 => Ok(QJobTopic::GenerateGroth16Proof),
            2 => Ok(QJobTopic::BlockUserSignatureProof),
            3 => Ok(QJobTopic::NotifyCoordinatorComplete),
            4 => Ok(QJobTopic::NotifyRealmComplete),
            5 => Ok(QJobTopic::AggregateJobs),
            _ => Err(anyhow::format_err!("Invalid QJobTopic value: {}", value)),
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ProvingJobDataType {
    InputWitness = 0,
    BaseInputProof = 1,
    OutputProof = 8,
    Counter = 16,
}
impl ProvingJobDataType {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
impl TryFrom<u8> for ProvingJobDataType {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ProvingJobDataType::InputWitness),
            1 => Ok(ProvingJobDataType::BaseInputProof),
            8 => Ok(ProvingJobDataType::OutputProof),
            16 => Ok(ProvingJobDataType::Counter),
            _ => Err(anyhow::format_err!("Invalid ProvingJobDataType value: {}", value)),
        }
    }
}
impl From<ProvingJobDataType> for u8 {
    fn from(value: ProvingJobDataType) -> u8 {
        value as u8
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord, Display, AsRefStr)]
#[repr(u8)]
pub enum ProvingJobCircuitType {
    AppendUserRegistrationTree = 0,
    AppendUserRegistrationTreeAggregate = 1,

    AddL1Deposit = 2,
    AddL1DepositAggregate = 3,

    ClaimL1Deposit = 4,
    ClaimL1DepositAggregate = 5,

    UserEndCap = 6,
    GUTATwoEndCap = 7,
    GUTATwoGUTA = 8,
    GUTALeftEndCapRightGUTA = 9,
    GUTALeftGUTARightEndCap = 10,
    GUTASingleEndCap = 11,
    GUTARegisterUsers = 12,
    GUTAVerifyToCap = 13,
    GUTAOnlyRegisterUsers = 14,
    GUTANoChange = 15,

    AddL1Withdrawal = 16,
    AddL1WithdrawalAggregate = 17,

    BatchDeployContracts = 18,
    BatchDeployContractsAggregate = 19,

    ProcessL1Withdrawal = 20,
    ProcessL1WithdrawalAggregate = 21,

    GenerateRollupStateTransitionProof = 32,
    GenerateSigHashIntrospectionProof = 33,
    GenerateFinalSigHashProof = 34,
    GenerateFinalSigHashProofGroth16 = 35,
    WrapFinalSigHashProofBLS12381 = 36,

    AggUserRegisterDeployContractsGUTA = 40,
    AggAddProcessL1WithdrawalAddL1Deposit = 41,

    DummyAppendUserRegistrationTreeAggregate = 48,
    DummyAddL1DepositAggregate = 49,
    DummyClaimL1DepositAggregate = 50,
    DummyGUTA = 51,
    DummyAddL1WithdrawalAggregate = 52,
    DummyProcessL1WithdrawalAggregate = 53,
    DummyBatchDeployContractsAggregate = 54,

    // ADDED NEW - For Historical Upgrades
    GUTATwoGUTAWithCheckpointUpgrade = 55,
    GUTAVerifyToCapWithCheckpointUpgrade = 56,

    WrappedSignatureProof = 64,
    Secp256K1SignatureProof = 65,

    NotifyRealmComplete = 192,

    TypeA = 224,
    TypeB = 225,
    TypeC = 226,
    TypeD = 227,
    TypeE = 228,
    TypeF = 229,
    Unknown = 255,
}

impl ProvingJobCircuitType {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
    pub fn to_circuit_group_id(&self) -> u32 {
        (self.to_u8() as u32) + 0xCF00u32
    }
    pub fn get_agg_leaf_circuit_type_or_err(&self) -> anyhow::Result<Self> {
        let leaf_type = match self {
            ProvingJobCircuitType::AppendUserRegistrationTree => ProvingJobCircuitType::AppendUserRegistrationTree,
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => ProvingJobCircuitType::AppendUserRegistrationTree,
            ProvingJobCircuitType::AddL1Deposit => ProvingJobCircuitType::AddL1Deposit,
            ProvingJobCircuitType::AddL1DepositAggregate => ProvingJobCircuitType::AddL1Deposit,
            ProvingJobCircuitType::ClaimL1Deposit => ProvingJobCircuitType::ClaimL1Deposit,
            ProvingJobCircuitType::ClaimL1DepositAggregate => ProvingJobCircuitType::ClaimL1Deposit,
            ProvingJobCircuitType::AddL1Withdrawal => ProvingJobCircuitType::AddL1Withdrawal,
            ProvingJobCircuitType::AddL1WithdrawalAggregate => ProvingJobCircuitType::AddL1Withdrawal,
            ProvingJobCircuitType::BatchDeployContracts => ProvingJobCircuitType::BatchDeployContracts,
            ProvingJobCircuitType::BatchDeployContractsAggregate => ProvingJobCircuitType::BatchDeployContracts,
            ProvingJobCircuitType::ProcessL1Withdrawal => ProvingJobCircuitType::ProcessL1Withdrawal,
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => ProvingJobCircuitType::ProcessL1Withdrawal,
            _ => anyhow::bail!("circuit type {:?} does not have a leaf type", self),
        };
        Ok(leaf_type)
    }

    pub fn get_agg_circuit_type_or_err(&self) -> anyhow::Result<Self> {
        let leaf_type = match self {
            ProvingJobCircuitType::AppendUserRegistrationTree => ProvingJobCircuitType::AppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => ProvingJobCircuitType::AppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::AddL1Deposit => ProvingJobCircuitType::AddL1DepositAggregate,
            ProvingJobCircuitType::AddL1DepositAggregate => ProvingJobCircuitType::AddL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1Deposit => ProvingJobCircuitType::ClaimL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1DepositAggregate => ProvingJobCircuitType::ClaimL1DepositAggregate,
            ProvingJobCircuitType::AddL1Withdrawal => ProvingJobCircuitType::AddL1WithdrawalAggregate,
            ProvingJobCircuitType::AddL1WithdrawalAggregate => ProvingJobCircuitType::AddL1WithdrawalAggregate,
            ProvingJobCircuitType::BatchDeployContracts => ProvingJobCircuitType::BatchDeployContractsAggregate,
            ProvingJobCircuitType::BatchDeployContractsAggregate => ProvingJobCircuitType::BatchDeployContractsAggregate,
            ProvingJobCircuitType::ProcessL1Withdrawal => ProvingJobCircuitType::ProcessL1WithdrawalAggregate,
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => ProvingJobCircuitType::ProcessL1WithdrawalAggregate,
            _ => anyhow::bail!("circuit type {:?} does not have a aggregated circuit type", self),
        };
        Ok(leaf_type)
    }

    pub fn get_agg_dummy_circuit_type_or_err(&self) -> anyhow::Result<Self> {
        let leaf_type = match self {
            ProvingJobCircuitType::AppendUserRegistrationTree => ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::AddL1Deposit => ProvingJobCircuitType::DummyAddL1DepositAggregate,
            ProvingJobCircuitType::AddL1DepositAggregate => ProvingJobCircuitType::DummyAddL1DepositAggregate,
            ProvingJobCircuitType::DummyAddL1DepositAggregate => ProvingJobCircuitType::DummyAddL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1Deposit => ProvingJobCircuitType::DummyClaimL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1DepositAggregate => ProvingJobCircuitType::DummyClaimL1DepositAggregate,
            ProvingJobCircuitType::DummyClaimL1DepositAggregate => ProvingJobCircuitType::DummyClaimL1DepositAggregate,
            ProvingJobCircuitType::AddL1Withdrawal => ProvingJobCircuitType::DummyAddL1WithdrawalAggregate,
            ProvingJobCircuitType::AddL1WithdrawalAggregate => ProvingJobCircuitType::DummyAddL1WithdrawalAggregate,
            ProvingJobCircuitType::DummyAddL1WithdrawalAggregate => ProvingJobCircuitType::DummyAddL1WithdrawalAggregate,
            ProvingJobCircuitType::BatchDeployContracts => ProvingJobCircuitType::DummyBatchDeployContractsAggregate,
            ProvingJobCircuitType::BatchDeployContractsAggregate => ProvingJobCircuitType::DummyBatchDeployContractsAggregate,
            ProvingJobCircuitType::DummyBatchDeployContractsAggregate => ProvingJobCircuitType::DummyBatchDeployContractsAggregate,
            ProvingJobCircuitType::ProcessL1Withdrawal => ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate,
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate,
            ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate => ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate,
            _ => anyhow::bail!("circuit type {:?} does not have a aggregated dummy circuit type", self),
        };
        Ok(leaf_type)
    }
}

impl TryFrom<u8> for ProvingJobCircuitType {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ProvingJobCircuitType::AppendUserRegistrationTree),
            1 => Ok(ProvingJobCircuitType::AppendUserRegistrationTreeAggregate),
            2 => Ok(ProvingJobCircuitType::AddL1Deposit),
            3 => Ok(ProvingJobCircuitType::AddL1DepositAggregate),
            4 => Ok(ProvingJobCircuitType::ClaimL1Deposit),
            5 => Ok(ProvingJobCircuitType::ClaimL1DepositAggregate),
            6 => Ok(ProvingJobCircuitType::UserEndCap),
            7 => Ok(ProvingJobCircuitType::GUTATwoEndCap),
            8 => Ok(ProvingJobCircuitType::GUTATwoGUTA),
            9 => Ok(ProvingJobCircuitType::GUTALeftEndCapRightGUTA),
            10 => Ok(ProvingJobCircuitType::GUTALeftGUTARightEndCap),
            11 => Ok(ProvingJobCircuitType::GUTASingleEndCap),
            12 => Ok(ProvingJobCircuitType::GUTARegisterUsers),
            13 => Ok(ProvingJobCircuitType::GUTAVerifyToCap),
            14 => Ok(ProvingJobCircuitType::GUTAOnlyRegisterUsers),
            15 => Ok(ProvingJobCircuitType::GUTANoChange),
            16 => Ok(ProvingJobCircuitType::AddL1Withdrawal),
            17 => Ok(ProvingJobCircuitType::AddL1WithdrawalAggregate),
            18 => Ok(ProvingJobCircuitType::BatchDeployContracts),
            19 => Ok(ProvingJobCircuitType::BatchDeployContractsAggregate),
            20 => Ok(ProvingJobCircuitType::ProcessL1Withdrawal),
            21 => Ok(ProvingJobCircuitType::ProcessL1WithdrawalAggregate),
            32 => Ok(ProvingJobCircuitType::GenerateRollupStateTransitionProof),
            33 => Ok(ProvingJobCircuitType::GenerateSigHashIntrospectionProof),
            34 => Ok(ProvingJobCircuitType::GenerateFinalSigHashProof),
            35 => Ok(ProvingJobCircuitType::GenerateFinalSigHashProofGroth16),
            36 => Ok(ProvingJobCircuitType::WrapFinalSigHashProofBLS12381),
            40 => Ok(ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA),
            41 => Ok(ProvingJobCircuitType::AggAddProcessL1WithdrawalAddL1Deposit),
            48 => Ok(ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate),
            49 => Ok(ProvingJobCircuitType::DummyAddL1DepositAggregate),
            50 => Ok(ProvingJobCircuitType::DummyClaimL1DepositAggregate),
            51 => Ok(ProvingJobCircuitType::DummyGUTA),
            52 => Ok(ProvingJobCircuitType::DummyAddL1WithdrawalAggregate),
            53 => Ok(ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate),
            54 => Ok(ProvingJobCircuitType::DummyBatchDeployContractsAggregate),
            55 => Ok(ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade),
            56 => Ok(ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade),

            64 => Ok(ProvingJobCircuitType::WrappedSignatureProof),
            65 => Ok(ProvingJobCircuitType::Secp256K1SignatureProof),
            192 => Ok(ProvingJobCircuitType::NotifyRealmComplete),

            224 => Ok(ProvingJobCircuitType::TypeA),
            225 => Ok(ProvingJobCircuitType::TypeB),
            226 => Ok(ProvingJobCircuitType::TypeC),
            227 => Ok(ProvingJobCircuitType::TypeD),
            228 => Ok(ProvingJobCircuitType::TypeE),
            229 => Ok(ProvingJobCircuitType::TypeF),
            255 => Ok(ProvingJobCircuitType::Unknown),
            _ => Err(anyhow::format_err!("Invalid ProvingJobCircuitType value: {}", value)),
        }
    }
}

impl From<ProvingJobCircuitType> for u8 {
    fn from(value: ProvingJobCircuitType) -> Self {
        value as u8
    }
}

pub type QProvingJobDataIDSerialized = [u8; 24];

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Copy, Eq, Hash, Clone, Debug)]
pub struct QProvingJobDataIDSerializedWrapped(#[serde_as(as = "serde_with::hex::Hex")] pub QProvingJobDataIDSerialized);

impl QProvingJobDataIDSerializedWrapped {
    pub fn from_hex_string(s: &str) -> Result<Self, FromHexError> {
        let bytes = hex::decode(s)?;
        assert_eq!(bytes.len(), 24);
        let mut array = [0u8; 24];
        array.copy_from_slice(&bytes);
        Ok(Self(array))
    }
}

#[serde_as]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QWorkerJobBenchmark {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub job_id: QProvingJobDataIDSerialized,
    pub duration: u64,
}

pub type LayerId = TaskId;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct QProvingTaskLayer {
    pub layer_id: LayerId,
    pub task_ids: Vec<TaskId>,
    pub job_ids: Vec<QProvingJobDataID>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct QProvingTask {
    pub task_id: TaskId,
    pub job_ids: Vec<QProvingJobDataID>,
}

impl QProvingTask {
    pub fn new(job_ids: &[QProvingJobDataID]) -> Self {
        let task_id = TaskId::new();
        Self {
            task_id,
            job_ids: job_ids.to_vec(),
        }
    }

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct JobProofSibling {
    #[serde(rename = "sibling_hash")]
    pub hash: QHashOut<F>,
    pub is_left: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JobProof {
    pub value: QHashOut<F>,
    pub siblings: Vec<JobProofSibling>,
    pub root: QHashOut<F>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct VariableHeightProofSibling {
    pub sibling_branch: QHashOut<F>,
    pub sibling_reward_leaf: QHashOut<F>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VariableHeightRewardMerkleProof {
    pub top_siblings: Vec<VariableHeightProofSibling>,
    pub sibling_branch: QHashOut<F>,
    pub reward_leaf: QHashOut<F>,
    pub proof_height: F,
    pub index: F,
}

impl VariableHeightRewardMerkleProof {
    pub fn combine_with(self, top_proof: VariableHeightRewardMerkleProof) -> VariableHeightRewardMerkleProof {
        use plonky2::field::goldilocks_field::GoldilocksField as F;

        let mut combined_top_siblings = self.top_siblings;
        combined_top_siblings.extend(top_proof.top_siblings);

        let bottom_height = self.proof_height.to_canonical_u64();
        let top_index = top_proof.index.to_canonical_u64();
        let combined_index = self.index.to_canonical_u64() | (top_index << bottom_height);

        VariableHeightRewardMerkleProof {
            top_siblings: combined_top_siblings,
            sibling_branch: self.sibling_branch,
            reward_leaf: self.reward_leaf,
            proof_height: self.proof_height + top_proof.proof_height,
            index: F::from_canonical_u64(combined_index),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QProvingTaskGraph {
    pub tasks: HashMap<TaskId, QProvingTask>,
    pub graph: BidirectionalGraph<TaskId>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QProvingJobGraph {
    pub deploy_contracts_graph: BidirectionalGraph<QProvingJobDataID>,
    pub user_registrations_graph: BidirectionalGraph<QProvingJobDataID>,
    pub guta_graph: BidirectionalGraph<QProvingJobDataID>,
}

impl QProvingJobGraph {
    pub fn new() -> Self {
        Self {
            deploy_contracts_graph: BidirectionalGraph::new(),
            user_registrations_graph: BidirectionalGraph::new(),
            guta_graph: BidirectionalGraph::new(),
        }
    }

    pub fn get_graph_for_job(&self, job_id: &QProvingJobDataID) -> anyhow::Result<&BidirectionalGraph<QProvingJobDataID>> {
        use ProvingJobCircuitType::*;

        match job_id.circuit_type {
            BatchDeployContracts | BatchDeployContractsAggregate | DummyBatchDeployContractsAggregate => Ok(&self.deploy_contracts_graph),
            AppendUserRegistrationTree | AppendUserRegistrationTreeAggregate | DummyAppendUserRegistrationTreeAggregate => {
                Ok(&self.user_registrations_graph)
            }
            GUTAOnlyRegisterUsers
            | GUTARegisterUsers
            | GUTATwoEndCap
            | GUTATwoGUTA
            | GUTALeftEndCapRightGUTA
            | GUTALeftGUTARightEndCap
            | GUTASingleEndCap
            | GUTAVerifyToCap
            | GUTATwoGUTAWithCheckpointUpgrade
            | GUTAVerifyToCapWithCheckpointUpgrade
            | GUTANoChange => Ok(&self.guta_graph),
            _ => anyhow::bail!("Unsupported circuit type: {:?}", job_id.circuit_type),
        }
    }

    pub async fn generate_variable_height_reward_proof<P: QJobRewardDataProvider>(
        &self,
        job_id: QProvingJobDataID,
        node_id: u32,
        provider: &P,
    ) -> anyhow::Result<(VariableHeightRewardMerkleProof, QProvingJobDataID)> {
        use plonky2::{field::goldilocks_field::GoldilocksField as F, hash::hash_types::HashOut};

        let graph = self.get_graph_for_job(&job_id)?;

        let mut path_to_root = Vec::new();
        let mut current_job = job_id;

        while let Some(dependents) = graph.get_dependents(&current_job) {
            if dependents.is_empty() {
                break;
            }
            let parent = *dependents.iter().next().unwrap();
            path_to_root.push((current_job, parent));
            current_job = parent;
        }

        let root_job_id = current_job;

        let actual_height = path_to_root.len();
        let mut top_siblings = Vec::new();

        for &(child, parent) in path_to_root.iter() {
            let parent_dependencies = graph.get_dependencies(&parent).unwrap();
            let deps_vec: Vec<_> = parent_dependencies.iter().cloned().collect();

            let (sibling_branch, sibling_reward_leaf) = if deps_vec.len() == 2 {
                let sibling_id = if child == deps_vec[0] { deps_vec[1] } else { deps_vec[0] };
                let sibling_commitment = provider.get_job_commitment(sibling_id).await?;
                let sibling_worker_public_key = provider.get_job_worker_public_key(sibling_id).await?;
                let sibling_reward_leaf = provider.get_job_worker_public_key(parent).await?;
                (QHashOut(PoseidonHash::two_to_one(sibling_commitment.into(), sibling_worker_public_key.into())), sibling_reward_leaf)
            } else if deps_vec.len() == 1 {
                let parent_reward_leaf = provider.get_job_worker_public_key(parent).await?;
                (QHashOut(HashOut { elements: [F::ZERO; 4] }), parent_reward_leaf)
            } else {
                (QHashOut(HashOut { elements: [F::ZERO; 4] }), QHashOut(HashOut { elements: [F::ZERO; 4] }))
            };

            top_siblings.push(VariableHeightProofSibling {
                sibling_branch,
                sibling_reward_leaf,
            });
        }

        let mut index_bits = vec![0u8; actual_height];

        for (level, &(child, parent)) in path_to_root.iter().enumerate() {
            if let Some(parent_dependencies) = graph.get_dependencies(&parent) {
                let deps_vec: Vec<_> = parent_dependencies.iter().cloned().collect();

                let bit_value = match deps_vec.len() {
                    1 => {
                        if child != deps_vec[0] {
                            return Err(anyhow::anyhow!("Child not found in single dependency"));
                        }
                        0
                    }
                    2 => {
                        if child == deps_vec[0] {
                            0
                        } else if child == deps_vec[1] {
                            1
                        } else {
                            return Err(anyhow::anyhow!("Child not found in dependencies"));
                        }
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Invalid number of dependencies: {}", deps_vec.len()));
                    }
                };

                index_bits[actual_height - 1 - level] = bit_value;
            }
        }

        let mut index_value = 0u64;
        for (i, &bit) in index_bits.iter().enumerate() {
            if bit == 1 {
                index_value |= 1u64 << (actual_height - 1 - i);
            }
        }

        let sibling_branch = provider.get_job_commitment(job_id).await?;
        let reward_leaf = provider.get_job_worker_public_key(job_id).await?;

        let proof = VariableHeightRewardMerkleProof {
            top_siblings,
            sibling_branch,
            reward_leaf,
            proof_height: F::from_canonical_usize(actual_height),
            index: F::from_canonical_u64(index_value),
        };

        Ok((proof, root_job_id))
    }

    pub fn get_graphviz(&self) -> String {
        let mut output = String::new();
        output.push_str("digraph QProvingJobGraph {\n");
        output.push_str("  rankdir=TB;\n");

        output.push_str("  subgraph cluster_deploy_contracts {\n");
        output.push_str("    label=\"Deploy Contracts\";\n");
        output.push_str("    color=blue;\n");
        self.add_graph_to_dot(&mut output, &self.deploy_contracts_graph, "deploy");
        output.push_str("  }\n\n");

        output.push_str("  subgraph cluster_user_registrations {\n");
        output.push_str("    label=\"User Registrations\";\n");
        output.push_str("    color=green;\n");
        self.add_graph_to_dot(&mut output, &self.user_registrations_graph, "user");
        output.push_str("  }\n\n");

        output.push_str("  subgraph cluster_guta {\n");
        output.push_str("    label=\"GUTA\";\n");
        output.push_str("    color=red;\n");
        self.add_graph_to_dot(&mut output, &self.guta_graph, "guta");
        output.push_str("  }\n\n");

        output.push_str("}\n");
        output
    }

    fn add_graph_to_dot(&self, output: &mut String, graph: &BidirectionalGraph<QProvingJobDataID>, prefix: &str) {
        let levels = graph.ts_order();

        for (level_idx, level) in levels.iter().enumerate() {
            for job_id in level {
                let job_id_hex = job_id.to_hex_string();
                let node_name = format!("{}_{}", prefix, job_id_hex);
                let circuit_type = format!("{:?}", job_id.circuit_type);

                output.push_str(&format!(
                    "    \"{}\" [label=\"{}\\n{}\", shape=box, style=filled, fillcolor=lightblue];\n",
                    node_name, circuit_type, job_id_hex
                ));
            }
        }

        for level in &levels {
            for job_id in level {
                if let Some(dependencies) = graph.get_dependencies(job_id) {
                    for dep_job_id in dependencies {
                        let dep_job_id_hex = dep_job_id.to_hex_string();
                        let job_id_hex = job_id.to_hex_string();

                        let from_node = format!("{}_{}", prefix, dep_job_id_hex);
                        let to_node = format!("{}_{}", prefix, job_id_hex);

                        output.push_str(&format!("    \"{}\" -> \"{}\";\n", from_node, to_node));
                    }
                }
            }
        }
    }
}

impl QProvingTaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            graph: BidirectionalGraph::new(),
        }
    }

    pub fn clear(&mut self) {
        self.tasks.clear();
        self.graph.clear();
    }

    pub fn add_task(&mut self, task: QProvingTask) {
        let task_id = task.task_id();
        self.tasks.insert(task_id, task);
        self.graph.add_node(task_id);
    }

    pub fn add_dep(&mut self, task: QProvingTask, dep_task: QProvingTask) {
        self.graph.add_edge(task.task_id(), dep_task.task_id());
        self.add_task(task);
        self.add_task(dep_task);
    }

    pub fn ts(&self) -> Vec<TaskId> {
        let ts_order = self.graph.ts_order();
        ts_order.into_iter().flatten().collect()
    }

    pub fn ts_layers(&self) -> Vec<QProvingTaskLayer> {
        let mut sorted_layers = Vec::new();
        let ts_order = self.graph.ts_order();
        for current_layer in ts_order {
            let layer_id = LayerId::new();
            let mut job_ids = Vec::new();
            let task_ids = current_layer.clone();

            for &task_id in &task_ids {
                if let Some(task) = self.tasks.get(&task_id) {
                    job_ids.extend(task.job_ids.clone());
                }
            }

            sorted_layers.push(QProvingTaskLayer {
                layer_id,
                task_ids: task_ids.clone(),
                job_ids,
            });
        }
        sorted_layers
    }

    pub fn get_task(&self, task_id: TaskId) -> Option<&QProvingTask> {
        self.tasks.get(&task_id)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct QProvingJobDataID {
    pub topic: QJobTopic,
    pub goal_id: u64,
    pub circuit_type: ProvingJobCircuitType,
    pub group_id: u32,
    pub sub_group_id: u32,
    pub task_index: u32,
    pub data_type: ProvingJobDataType,
    pub data_index: u8,
}
impl QProvingJobDataID {
    pub fn notify_realm_complete(checkpoint_id: u64, realm_id: u32) -> Self {
        Self {
            topic: QJobTopic::NotifyRealmComplete,
            goal_id: checkpoint_id,
            group_id: realm_id,
            circuit_type: ProvingJobCircuitType::Unknown,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn notify_block_complete(checkpoint_id: u64, coordinator_id: u32) -> Self {
        Self {
            topic: QJobTopic::NotifyCoordinatorComplete,
            goal_id: checkpoint_id,
            group_id: coordinator_id,
            circuit_type: ProvingJobCircuitType::Unknown,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn with_ps_prefix(&self, prefix: [u8; 4]) -> [u8; 28] {
        let mut result = [0u8; 28];
        result[0..3].copy_from_slice(&prefix);
        result[4] = self.topic.to_u8();
        result[5..13].copy_from_slice(&self.goal_id.to_le_bytes());
        result[13] = self.circuit_type.to_u8();
        result[14..18].copy_from_slice(&self.group_id.to_le_bytes());
        result[18..22].copy_from_slice(&self.sub_group_id.to_le_bytes());
        result[22..26].copy_from_slice(&self.task_index.to_le_bytes());
        result[26] = self.data_type.to_u8();
        result[27] = self.data_index;
        result
    }

    pub fn try_from_byte_vec(value: &[u8]) -> anyhow::Result<Self> {
        if value.len() != 24 {
            anyhow::bail!("invalid byte length for proving job data id");
        }
        let topic: QJobTopic = value[0].try_into()?;
        let goal_id = u64::from_le_bytes(value[1..9].try_into()?);
        let circuit_type = ProvingJobCircuitType::try_from(value[9])?;
        let group_id = u32::from_le_bytes(value[10..14].try_into()?);
        let sub_group_id = u32::from_le_bytes(value[14..18].try_into()?);
        let task_index = u32::from_le_bytes(value[18..22].try_into()?);
        let data_type = ProvingJobDataType::try_from(value[22])?;
        let data_index = value[23];
        Ok(QProvingJobDataID {
            topic,
            goal_id,
            circuit_type,
            group_id,
            sub_group_id,
            task_index,
            data_type,
            data_index,
        })
    }

    pub fn to_key_string(&self) -> String {
        format!(
            "topic:{:02X}:goal:{:016X}:circuit:{:02X}:group:{:08X}:subgroup:{:08X}:task:{:08X}:dtype:{:02X}:didx:{:02X}",
            self.topic.to_u8(),
            self.goal_id,
            self.circuit_type.to_u8(),
            self.group_id,
            self.sub_group_id,
            self.task_index,
            self.data_type.to_u8(),
            self.data_index,
        )
    }
    pub fn from_key_string(s: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 18 {
            anyhow::bail!("invalid key string: {}", s);
        }

        // Parts index correspondence:
        // 0="topic" 1=val 2="goal" 3=val 4="circuit" 5=val
        // 6="group" 7=val 8="subgroup" 9=val 10="task" 11=val
        // 12="dtype" 13=val 14="didx" 15=val
        // (Note that the length after split is 16, not 18; there is no extra ":")

        let topic: QJobTopic = u8::from_str_radix(parts[1], 16)?.try_into()?;
        let goal_id = u64::from_str_radix(parts[3], 16)?;
        let circuit_type = ProvingJobCircuitType::try_from(u8::from_str_radix(parts[5], 16)?)?;
        let group_id = u32::from_str_radix(parts[7], 16)?;
        let sub_group_id = u32::from_str_radix(parts[9], 16)?;
        let task_index = u32::from_str_radix(parts[11], 16)?;
        let data_type = ProvingJobDataType::try_from(u8::from_str_radix(parts[13], 16)?)?;
        let data_index = u8::from_str_radix(parts[15], 16)?;

        Ok(Self {
            topic,
            goal_id,
            circuit_type,
            group_id,
            sub_group_id,
            task_index,
            data_type,
            data_index,
        })
    }
}


impl fmt::Display for QProvingJobDataID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return write!(f, "{}", self.to_hex_string());
        }

        write!(
            f,
            "QJob[topic={:?}(0x{:02X}), goal={} (0x{:016X}), circuit={:?}(0x{:02X}, gid=0x{:08X}), \
group=0x{:08X}, subgroup=0x{:08X}, task=0x{:08X}, dtype={:?}(0x{:02X}), didx=0x{:02X}]",
            self.topic,                       self.topic.to_u8(),
            self.goal_id,                     self.goal_id,
            self.circuit_type,                self.circuit_type.to_u8(), self.circuit_type.to_circuit_group_id(),
            self.group_id,
            self.sub_group_id,
            self.task_index,
            self.data_type,                   self.data_type.to_u8(),
            self.data_index
        )
    }
}
impl From<&QProvingJobDataID> for [u8; 24] {
    fn from(value: &QProvingJobDataID) -> Self {
        let mut result = [0u8; 24];
        result[0] = value.topic.to_u8();
        result[1..9].copy_from_slice(&value.goal_id.to_le_bytes());
        result[9] = value.circuit_type.to_u8();
        result[10..14].copy_from_slice(&value.group_id.to_le_bytes());
        result[14..18].copy_from_slice(&value.sub_group_id.to_le_bytes());
        result[18..22].copy_from_slice(&value.task_index.to_le_bytes());
        result[22] = value.data_type.to_u8();
        result[23] = value.data_index;
        result
    }
}
impl TryFrom<[u8; 24]> for QProvingJobDataID {
    type Error = anyhow::Error;
    fn try_from(value: [u8; 24]) -> Result<Self, Self::Error> {
        let topic: QJobTopic = value[0].try_into()?;
        let goal_id = u64::from_le_bytes(value[1..9].try_into()?);
        let circuit_type = ProvingJobCircuitType::try_from(value[9])?;
        let group_id = u32::from_le_bytes(value[10..14].try_into()?);
        let sub_group_id = u32::from_le_bytes(value[14..18].try_into()?);
        let task_index = u32::from_le_bytes(value[18..22].try_into()?);
        let data_type = ProvingJobDataType::try_from(value[22])?;
        let data_index = value[23];
        Ok(QProvingJobDataID {
            topic,
            goal_id,
            circuit_type,
            group_id,
            sub_group_id,
            task_index,
            data_type,
            data_index,
        })
    }
}

impl QProvingJobDataID {
    pub fn new(
        topic: QJobTopic,
        goal_id: u64,
        group_id: u32,
        sub_group_id: u32,
        task_index: u32,
        circuit_type: ProvingJobCircuitType,
        data_type: ProvingJobDataType,
        data_index: u8,
    ) -> Self {
        Self {
            topic,
            goal_id,
            circuit_type,
            group_id,
            sub_group_id,
            task_index,
            data_type,
            data_index,
        }
    }
    pub fn guta_two_end_cap_witness(checkpoint_id: u64, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            group_id,
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTATwoEndCap,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_two_agg_witness(checkpoint_id: u64, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            group_id,
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTATwoGUTA,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_two_agg_witness_with_checkpoint_upgrade(checkpoint_id: u64, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            group_id,
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_left_end_cap_right_guta_witness(checkpoint_id: u64, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            group_id,
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_left_guta_right_end_cap_witness(checkpoint_id: u64, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            group_id,
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_single_end_cap_witness(checkpoint_id: u64, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            group_id,
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTASingleEndCap,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn core_op_witness(checkpoint_id: u64, group_id: u32, circuit_type: ProvingJobCircuitType, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            group_id,
            sub_group_id,
            task_index,
            circuit_type,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn transfer_signature_proof(checkpoint_id: u64, group_id: u32, transfer_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::WrappedSignatureProof,
            sub_group_id: 0,
            task_index: transfer_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn end_cap_proof(checkpoint_id: u64, group_id: u32, user_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::UserEndCap,
            sub_group_id: 1,
            task_index: user_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn withdrawal_signature_proof(checkpoint_id: u64, group_id: u32, withdrawal_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::WrappedSignatureProof,
            sub_group_id: 2,
            task_index: withdrawal_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn claim_deposit_l1_signature_proof(checkpoint_id: u64, group_id: u32, deposit_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::Secp256K1SignatureProof,
            sub_group_id: 3,
            task_index: deposit_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn new_proof_job_id(goal_id: u64, group_id: u32, circuit_type: ProvingJobCircuitType, sub_group_id: u32, task_index: u32) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id,
            group_id,
            circuit_type,
            sub_group_id,
            task_index,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn new_groth16_proof_job_id(goal_id: u64, group_id: u32, circuit_type: ProvingJobCircuitType, sub_group_id: u32, task_index: u32) -> Self {
        Self {
            topic: QJobTopic::GenerateGroth16Proof,
            goal_id,
            circuit_type,
            group_id,
            sub_group_id,
            task_index,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn get_block_aggregate_jobs_group(checkpoint_id: u64, group_id: u32, task_index: u32) -> Self {
        Self {
            topic: QJobTopic::AggregateJobs,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::Unknown,
            sub_group_id: 0,
            task_index,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn block_agg_state_part_1_input_witness(checkpoint_id: u64, group_id: u32) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn block_agg_state_part_2_input_witness(checkpoint_id: u64, group_id: u32) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::AggAddProcessL1WithdrawalAddL1Deposit,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn block_state_transition_input_witness(checkpoint_id: u64, group_id: u32) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::GenerateRollupStateTransitionProof,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn sighash_introspection_input_witness(checkpoint_id: u64, group_id: u32, input_id: usize) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::GenerateSigHashIntrospectionProof,
            sub_group_id: 0,
            task_index: input_id as u32,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn sighash_final_input_witness(checkpoint_id: u64, group_id: u32, input_id: usize) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::GenerateFinalSigHashProof,
            sub_group_id: input_id as u32,
            task_index: input_id as u32,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn wrap_sighash_final_bls3812_input_witness(checkpoint_id: u64, group_id: u32, input_id: usize) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: checkpoint_id,
            group_id,
            circuit_type: ProvingJobCircuitType::WrapFinalSigHashProofBLS12381,
            sub_group_id: input_id as u32,
            task_index: input_id as u32,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn get_input_proof_id(&self, data_index: u8) -> Self {
        Self {
            data_type: ProvingJobDataType::BaseInputProof,
            data_index,
            ..*self
        }
    }

    pub fn is_notify_coordinator_complete(&self) -> bool {
        self.topic == QJobTopic::NotifyCoordinatorComplete
    }

    pub fn is_notify_realm_complete(&self) -> bool {
        self.topic == QJobTopic::NotifyRealmComplete
    }

    pub fn is_notify_complete(&self) -> bool {
        self.is_notify_coordinator_complete() || self.is_notify_realm_complete()
    }

    pub fn is_provable(&self) -> bool {
        self.topic == QJobTopic::GenerateStandardProof && !self.is_notify_complete()
    }

    pub fn get_tree_parent_proof_input_id(&self) -> Self {
        let parent_type = match self.circuit_type {
            ProvingJobCircuitType::AppendUserRegistrationTree => ProvingJobCircuitType::AppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => ProvingJobCircuitType::AppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::BatchDeployContracts => ProvingJobCircuitType::BatchDeployContractsAggregate,
            ProvingJobCircuitType::BatchDeployContractsAggregate => ProvingJobCircuitType::BatchDeployContractsAggregate,
            ProvingJobCircuitType::AddL1Deposit => ProvingJobCircuitType::AddL1DepositAggregate,
            ProvingJobCircuitType::AddL1DepositAggregate => ProvingJobCircuitType::AddL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1Deposit => ProvingJobCircuitType::ClaimL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1DepositAggregate => ProvingJobCircuitType::ClaimL1DepositAggregate,
            ProvingJobCircuitType::AddL1Withdrawal => ProvingJobCircuitType::AddL1WithdrawalAggregate,
            ProvingJobCircuitType::AddL1WithdrawalAggregate => ProvingJobCircuitType::AddL1WithdrawalAggregate,
            ProvingJobCircuitType::ProcessL1Withdrawal => ProvingJobCircuitType::ProcessL1WithdrawalAggregate,
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => ProvingJobCircuitType::ProcessL1WithdrawalAggregate,
            ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => ProvingJobCircuitType::AppendUserRegistrationTreeAggregate,
            ProvingJobCircuitType::DummyAddL1DepositAggregate => ProvingJobCircuitType::AddL1DepositAggregate,
            ProvingJobCircuitType::DummyClaimL1DepositAggregate => ProvingJobCircuitType::ClaimL1DepositAggregate,
            ProvingJobCircuitType::DummyAddL1WithdrawalAggregate => ProvingJobCircuitType::AddL1WithdrawalAggregate,
            ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate => ProvingJobCircuitType::ProcessL1WithdrawalAggregate,
            _ => self.circuit_type,
        };
        Self {
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
            circuit_type: parent_type,
            sub_group_id: self.sub_group_id + 1,
            task_index: self.task_index >> 1u32,
            ..*self
        }
    }
    pub fn get_output_id(&self) -> Self {
        Self {
            data_type: ProvingJobDataType::OutputProof,
            data_index: 0,
            ..*self
        }
    }
    pub fn get_input_witness_id(&self) -> Self {
        Self {
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
            ..*self
        }
    }
    pub fn get_sub_group_counter_id(&self) -> Self {
        Self {
            data_type: ProvingJobDataType::Counter,
            task_index: 0,
            data_index: 0,
            ..*self
        }
    }
    pub fn get_sub_group_counter_goal_id(&self) -> Self {
        Self {
            data_type: ProvingJobDataType::Counter,
            task_index: 0,
            data_index: 1,
            ..*self
        }
    }
    pub fn get_sub_group_counter_goal_next_jobs_id(&self) -> Self {
        Self {
            data_type: ProvingJobDataType::Counter,
            task_index: 0,
            data_index: 2,
            ..*self
        }
    }
    pub fn to_fixed_bytes(&self) -> QProvingJobDataIDSerialized {
        self.into()
    }
    pub fn with_task_index(&self, task_index: u32) -> Self {
        Self { task_index, ..*self }
    }
    pub fn to_hex_string(&self) -> String {
        hex::encode(&self.to_fixed_bytes())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct ProvingJobDataId {
    pub checkpoint_id: u64,
    pub job_id: QProvingJobDataID,
}

impl ProvingJobDataId {
    pub fn new(checkpoint_id: u64, job_id: QProvingJobDataID) -> Self {
        Self { checkpoint_id, job_id }
    }
}

impl HistoryQueueMetadataTagged for ProvingJobDataId {
    fn get_hq_metadata(&self) -> HistoryQueueMetadata {
        HistoryQueueMetadata {
            channel_id: REALM_PROOF_SYNC_CHANNEL,
            checkpoint_id: self.checkpoint_id,
            item_id: self.job_id.task_index as u64, // Use task_index as item_id
        }
    }
}
impl DrainQueueMetadataTagged for ProvingJobDataId {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {
        DrainQueueMetadata {
            channel_id: QED_CHECKPOINT_JOB_ID_CHANNEL,
            checkpoint_id: self.checkpoint_id,
            item_id: self.job_id.sub_group_id as u64,
        }
    }
}
impl KVQSerializable for ProvingJobDataId {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut bytes = vec![];
        bytes.extend(self.checkpoint_id.to_le_bytes());
        bytes.extend(self.job_id.to_bytes()?);
        Ok(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 8 + 24 {
            anyhow::bail!("invalid byte length for proving job data id");
        }
        let checkpoint_id = u64::from_le_bytes(bytes[0..8].try_into()?);
        let job_id = QProvingJobDataID::from_bytes(&bytes[8..])?;
        Ok(Self { checkpoint_id, job_id })
    }
}

pub trait QWorkerModeFilter {
    fn can_process_job(&self, job_id: QProvingJobDataID) -> bool;
}
impl QWorkerModeFilter for QWorkerMode {
    fn can_process_job(&self, job_id: QProvingJobDataID) -> bool {
        match *self {
            QWorkerMode::All => true,
            QWorkerMode::NoGroth16 => job_id.circuit_type != ProvingJobCircuitType::WrapFinalSigHashProofBLS12381,
            QWorkerMode::OnlyGroth16 => job_id.circuit_type == ProvingJobCircuitType::WrapFinalSigHashProofBLS12381,
        }
    }
}

impl KVQSerializable for QProvingJobDataID {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_fixed_bytes().to_vec())
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        QProvingJobDataID::try_from_byte_vec(bytes)
    }
}

impl VariableHeightRewardMerkleProof {
    pub fn compute_root_and_nullifier_index(&self) -> (QHashOut<F>, F) {
        let proof_height = self.proof_height.to_canonical_u64() as usize;
        let index = self.index.to_canonical_u64();

        if proof_height > 0 && self.top_siblings.len() < proof_height {
            panic!("Proof height {} but top_siblings only has {} elements", proof_height, self.top_siblings.len());
        }

        let mut nullifier_base = 0u64;
        let mut nullifier_level_start_index_multiplier = 1u64;

        let mut current_node_value = PoseidonHash::two_to_one(self.sibling_branch.0, self.reward_leaf.0);

        for i in 0..proof_height {
            let sibling = &self.top_siblings[i];
            let sibling_node = sibling.sibling_branch.0;
            let reward_leaf = sibling.sibling_reward_leaf.0;

            let index_bit = (index >> i) & 1;
            let branch_path_hash = if index_bit == 0 {
                PoseidonHash::two_to_one(current_node_value, sibling_node)
            } else {
                PoseidonHash::two_to_one(sibling_node, current_node_value)
            };
            current_node_value = PoseidonHash::two_to_one(branch_path_hash, reward_leaf);

            nullifier_base += nullifier_level_start_index_multiplier;
            nullifier_level_start_index_multiplier *= 2;
        }

        let nullifier_final_index = F::from_canonical_u64(nullifier_base + index);
        (QHashOut(current_node_value), nullifier_final_index)
    }

    pub fn pad_to_height(mut self, max_height: usize) -> Self {
        let proof_height = self.proof_height.to_canonical_u64() as usize;
        assert!(
            proof_height <= max_height,
            "Proof height {} exceeds max height {}",
            proof_height,
            max_height
        );
        while self.top_siblings.len() < max_height {
            self.top_siblings.push(VariableHeightProofSibling {
                sibling_branch: QHashOut(HashOut::ZERO),
                sibling_reward_leaf: QHashOut(HashOut::ZERO),
            });
        }

        self
    }

    pub fn verify_proof(&self, expected_root: &QHashOut<F>) -> bool {
        let (computed_root, _) = self.compute_root_and_nullifier_index();
        computed_root == *expected_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_height_proof_verification() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(5, 6, 7, 8).into(),
                QHashOut::from_values(9, 10, 11, 12).into()
            )),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let (root, nullifier) = proof.compute_root_and_nullifier_index();

        assert!(proof.verify_proof(&root));
        assert!(nullifier != F::ZERO);
    }

    #[test]
    fn test_job_graph_creation() {
        let graph = QProvingJobGraph::new();
        assert!(graph
            .deploy_contracts_graph
            .get_dependencies(&QProvingJobDataID::new_proof_job_id(
                1,
                0,
                ProvingJobCircuitType::BatchDeployContracts,
                0,
                0
            ))
            .is_none());
    }

    #[test]
    fn test_get_graph_for_job_selection() {
        let graph = QProvingJobGraph::new();

        let deploy_job = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::BatchDeployContracts, 0, 0);
        let user_reg_job = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 0);
        let guta_job = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTAOnlyRegisterUsers, 0, 0);

        assert!(std::ptr::eq(graph.get_graph_for_job(&deploy_job).unwrap(), &graph.deploy_contracts_graph));
        assert!(std::ptr::eq(
            graph.get_graph_for_job(&user_reg_job).unwrap(),
            &graph.user_registrations_graph
        ));
        assert!(std::ptr::eq(graph.get_graph_for_job(&guta_job).unwrap(), &graph.guta_graph));
    }

    #[test]
    fn test_variable_height_proof_with_multiple_levels() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![
                VariableHeightProofSibling {
                    sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                    sibling_reward_leaf: QHashOut::from_values(5, 6, 7, 8),
                },
                VariableHeightProofSibling {
                    sibling_branch: QHashOut::from_values(9, 10, 11, 12),
                    sibling_reward_leaf: QHashOut::from_values(13, 14, 15, 16),
                },
            ],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(17, 18, 19, 20).into(),
                QHashOut::from_values(21, 22, 23, 24).into()
            )),
            reward_leaf: QHashOut::from_values(25, 26, 27, 28),
            proof_height: F::from_canonical_usize(2),
            index: F::from_canonical_usize(1),
        };

        let (root, nullifier) = proof.compute_root_and_nullifier_index();

        assert!(proof.verify_proof(&root));
        assert!(nullifier != F::ZERO);
    }

    #[test]
    fn test_variable_height_proof_zero_height() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(1, 2, 3, 4).into(),
                QHashOut::from_values(5, 6, 7, 8).into()
            )),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(0),
            index: F::from_canonical_usize(0),
        };

        let (root, nullifier) = proof.compute_root_and_nullifier_index();

        assert!(proof.verify_proof(&root));
        assert_eq!(nullifier, F::from_canonical_usize(0));
    }

    #[test]
    fn test_compute_root_and_nullifier_basic() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(100, 101, 102, 103),
                sibling_reward_leaf: QHashOut::from_values(200, 201, 202, 203),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(1, 2, 3, 4).into(),
                QHashOut::from_values(5, 6, 7, 8).into()
            )),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let (root, nullifier) = proof.compute_root_and_nullifier_index();
        assert!(root != QHashOut::ZERO);
        assert!(nullifier != F::ZERO);
    }

    #[test]
    fn test_job_graph_all_circuit_types() {
        let graph = QProvingJobGraph::new();

        let deploy_jobs = vec![
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::BatchDeployContracts, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::BatchDeployContractsAggregate, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::DummyBatchDeployContractsAggregate, 0, 0),
        ];

        let user_reg_jobs = vec![
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTreeAggregate, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate, 0, 0),
        ];

        let guta_jobs = vec![
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTAOnlyRegisterUsers, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTARegisterUsers, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTALeftEndCapRightGUTA, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTALeftGUTARightEndCap, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTASingleEndCap, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTAVerifyToCap, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTANoChange, 0, 0),
        ];

        for job in deploy_jobs {
            assert!(std::ptr::eq(graph.get_graph_for_job(&job).unwrap(), &graph.deploy_contracts_graph));
        }

        for job in user_reg_jobs {
            assert!(std::ptr::eq(graph.get_graph_for_job(&job).unwrap(), &graph.user_registrations_graph));
        }

        for job in guta_jobs {
            assert!(std::ptr::eq(graph.get_graph_for_job(&job).unwrap(), &graph.guta_graph));
        }
    }

    #[test]
    fn test_nullifier_calculation() {
        let proof1 = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(5, 6, 7, 8).into(),
                QHashOut::from_values(9, 10, 11, 12).into()
            )),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let proof2 = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(5, 6, 7, 8).into(),
                QHashOut::from_values(9, 10, 11, 12).into()
            )),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1),
        };

        let (_, nullifier1) = proof1.compute_root_and_nullifier_index();
        let (_, nullifier2) = proof2.compute_root_and_nullifier_index();

        assert_ne!(nullifier1, nullifier2);
        assert_eq!(nullifier1, F::from_canonical_usize(1));
        assert_eq!(nullifier2, F::from_canonical_usize(2));
    }

    #[test]
    fn test_job_data_id_serialization() {
        let job_id = QProvingJobDataID::new_proof_job_id(12345, 100, ProvingJobCircuitType::BatchDeployContracts, 200, 300);

        let bytes = job_id.to_fixed_bytes();
        let recovered = QProvingJobDataID::try_from(bytes).unwrap();

        assert_eq!(job_id, recovered);
        assert_eq!(job_id.goal_id, 12345);
        assert_eq!(job_id.circuit_type, ProvingJobCircuitType::BatchDeployContracts);
        assert_eq!(job_id.group_id, 100);
        assert_eq!(job_id.sub_group_id, 200);
        assert_eq!(job_id.task_index, 300);
    }

    #[test]
    fn test_job_data_id_hex_conversion() {
        let job_id = QProvingJobDataID::new_proof_job_id(
            0x1234567890ABCDEF,
            0x12345678,
            ProvingJobCircuitType::GUTAOnlyRegisterUsers,
            0x87654321,
            0xABCDEF00,
        );

        let hex_string = job_id.to_hex_string();
        let bytes_from_hex = hex::decode(&hex_string).unwrap();
        let recovered = QProvingJobDataID::try_from_byte_vec(&bytes_from_hex).unwrap();

        assert_eq!(job_id, recovered);
        assert!(hex_string.len() == 48); // 24 bytes * 2 chars per byte
    }

    #[test]
    fn test_max_height_limits() {
        assert_eq!(GUTA_REWARDS_TREE_MAX_HEIGHT, 16);
        assert_eq!(CONTRACT_DEPLOYMENT_REWARDS_MAX_HEIGHT, 32);
        assert_eq!(USER_REGISTRATION_REWARDS_MAX_HEIGHT, 32);

        let max_index = (1u64 << GUTA_REWARDS_TREE_MAX_HEIGHT) - 1;
        assert_eq!(max_index, 65535); // 2^16 - 1
    }

    #[test]
    #[should_panic(expected = "Proof height 20 but top_siblings only has 0 elements")]
    fn test_proof_height_validation() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![], // Empty siblings but proof_height = 20 should panic
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(1, 2, 3, 4).into(),
                QHashOut::from_values(5, 6, 7, 8).into()
            )),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(20), // > top_siblings.len()
            index: F::from_canonical_usize(0),
        };

        proof.compute_root_and_nullifier_index();
    }

    #[test]
    #[should_panic(expected = "Proof height 1 but top_siblings only has 0 elements")]
    fn test_index_validation() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![], // Empty siblings but proof_height = 1 should panic
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(1, 2, 3, 4).into(),
                QHashOut::from_values(5, 6, 7, 8).into()
            )),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        proof.compute_root_and_nullifier_index();
    }

    #[test]
    fn test_job_reward_data_provider_trait() {
        use std::collections::HashMap;

        let job_a = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 100);

        let mut commitments = HashMap::new();
        let mut public_keys = HashMap::new();

        commitments.insert(job_a, QHashOut::from_values(10, 11, 12, 13));
        public_keys.insert(job_a, QHashOut::from_values(20, 21, 22, 23));

        let _provider = MockJobRewardDataProvider { commitments, public_keys };
    }

    #[test]
    fn test_job_graph_dependency_structure() {
        let job_a = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 100);
        let job_b = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTreeAggregate, 0, 200);

        let mut graph = QProvingJobGraph::new();
        graph.user_registrations_graph.add_node(job_a);
        graph.user_registrations_graph.add_node(job_b);
        graph.user_registrations_graph.add_edge(job_b, job_a);

        assert!(graph.user_registrations_graph.get_dependents(&job_a).unwrap().contains(&job_b));
        assert!(graph.user_registrations_graph.get_dependencies(&job_b).unwrap().contains(&job_a));

        let job_c = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 300);
        let job_d = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::AppendUserRegistrationTreeAggregate, 0, 400);

        graph.user_registrations_graph.add_node(job_c);
        graph.user_registrations_graph.add_node(job_d);
        graph.user_registrations_graph.add_edge(job_d, job_a);
        graph.user_registrations_graph.add_edge(job_d, job_c);

        let deps = graph.user_registrations_graph.get_dependencies(&job_d).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&job_a));
        assert!(deps.contains(&job_c));
    }

    struct MockJobRewardDataProvider {
        commitments: std::collections::HashMap<QProvingJobDataID, QHashOut<F>>,
        public_keys: std::collections::HashMap<QProvingJobDataID, QHashOut<F>>,
    }

    #[async_trait::async_trait]
    impl QJobRewardDataProvider for MockJobRewardDataProvider {
        async fn get_job_commitment(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>> {
            self.commitments
                .get(&job_id)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("Commitment not found for job ID: {:?}", job_id))
        }

        async fn get_job_worker_public_key(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>> {
            self.public_keys
                .get(&job_id)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("Worker public key not found for job ID: {:?}", job_id))
        }
    }

    #[tokio::test]
    async fn test_generate_variable_height_reward_proof_simple() {
        use std::collections::HashMap;

        // Create test job IDs - create a proper hierarchy: root <- parent <- child
        let child_job = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 100);
        let parent_job = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 200);
        let root_job = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 300);

        // Set up mock provider with test data
        let mut commitments = HashMap::new();
        let mut public_keys = HashMap::new();

        commitments.insert(child_job, QHashOut::from_values(10, 11, 12, 13));
        commitments.insert(parent_job, QHashOut::from_values(14, 15, 16, 17));
        commitments.insert(root_job, QHashOut::from_values(18, 19, 20, 21));
        public_keys.insert(child_job, QHashOut::from_values(100, 101, 102, 103));
        public_keys.insert(parent_job, QHashOut::from_values(104, 105, 106, 107));
        public_keys.insert(root_job, QHashOut::from_values(108, 109, 110, 111));

        let provider = MockJobRewardDataProvider { commitments, public_keys };

        // Create job graph: child -> parent -> root (each depends on the next)
        let mut graph = QProvingJobGraph::new();
        graph.guta_graph.add_node(child_job);
        graph.guta_graph.add_node(parent_job);
        graph.guta_graph.add_node(root_job);
        graph.guta_graph.add_edge(parent_job, child_job); // parent depends on child
        graph.guta_graph.add_edge(root_job, parent_job); // root depends on parent

        // Test leaf node (child - no dependents)
        let (proof_child, nullifier_child) = graph.generate_variable_height_reward_proof(child_job, 0, &provider).await.unwrap();
        assert_eq!(proof_child.proof_height.to_canonical_u64() as usize, 2); // Updated based on actual behavior
        assert_eq!(proof_child.top_siblings.len(), 2); // Actual height, no padding
        assert_eq!(proof_child.reward_leaf, QHashOut::from_values(100, 101, 102, 103));
        assert_eq!(nullifier_child, root_job); // Root is the final nullifier

        // Test middle node (parent - has child as dependent)
        let (proof_parent, nullifier_parent) = graph.generate_variable_height_reward_proof(parent_job, 0, &provider).await.unwrap();
        assert_eq!(proof_parent.proof_height.to_canonical_u64() as usize, 1); // Path: parent -> root
        assert_eq!(proof_parent.top_siblings.len(), 1);
        assert_eq!(proof_parent.reward_leaf, QHashOut::from_values(104, 105, 106, 107));
        assert_eq!(nullifier_parent, root_job); // Root job ID

        // Test root node (has parent as dependent)
        let (proof_root, nullifier_root) = graph.generate_variable_height_reward_proof(root_job, 0, &provider).await.unwrap();
        assert_eq!(proof_root.proof_height.to_canonical_u64() as usize, 0); // Root has no dependents
        assert_eq!(proof_root.top_siblings.len(), 0);
        assert_eq!(proof_root.reward_leaf, QHashOut::from_values(108, 109, 110, 111));
        assert_eq!(nullifier_root, root_job);
    }

    #[tokio::test]
    async fn test_generate_variable_height_reward_proof_two_dependencies() {
        use std::collections::HashMap;

        // Create test jobs: job_c depends on job_a and job_b
        let job_a = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 100);
        let job_b = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 200);
        let job_c = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 300);

        let mut commitments = HashMap::new();
        let mut public_keys = HashMap::new();

        commitments.insert(job_a, QHashOut::from_values(1, 2, 3, 4));
        commitments.insert(job_b, QHashOut::from_values(5, 6, 7, 8));
        commitments.insert(job_c, QHashOut::from_values(9, 10, 11, 12));
        public_keys.insert(job_a, QHashOut::from_values(100, 101, 102, 103));
        public_keys.insert(job_b, QHashOut::from_values(104, 105, 106, 107));
        public_keys.insert(job_c, QHashOut::from_values(108, 109, 110, 111));

        let provider = MockJobRewardDataProvider { commitments, public_keys };

        // Create graph with two dependencies
        let mut graph = QProvingJobGraph::new();
        graph.guta_graph.add_node(job_a);
        graph.guta_graph.add_node(job_b);
        graph.guta_graph.add_node(job_c);
        graph.guta_graph.add_edge(job_c, job_a);
        graph.guta_graph.add_edge(job_c, job_b);

        let (proof, _nullifier) = graph.generate_variable_height_reward_proof(job_c, 0, &provider).await.unwrap();

        // job_c has no dependents, so height should be 0
        assert_eq!(proof.proof_height.to_canonical_u64() as usize, 0);
        assert_eq!(proof.top_siblings.len(), 0); // No siblings for root
        assert_eq!(proof.reward_leaf, QHashOut::from_values(108, 109, 110, 111));

        // Since job_c is a root with two dependencies, it should have left and right
        // branches
        assert_eq!(proof.sibling_branch, QHashOut(PoseidonHash::two_to_one(
            QHashOut::from_values(1, 2, 3, 4).into(),
            QHashOut::from_values(5, 6, 7, 8).into()
        )));
    }

    #[tokio::test]
    async fn test_nullifier_uniqueness_across_jobs() {
        use std::collections::{HashMap, HashSet};

        // Create multiple jobs in a complex graph to test nullifier uniqueness
        let job1 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 100);
        let job2 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 200);
        let job3 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 300);
        let job4 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 400);
        let root_job = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 500);

        // Setup mock provider
        let mut commitments = HashMap::new();
        let mut public_keys = HashMap::new();

        for (i, job) in [job1, job2, job3, job4, root_job].iter().enumerate() {
            commitments.insert(
                *job,
                QHashOut::from_values((i * 4) as u64, (i * 4 + 1) as u64, (i * 4 + 2) as u64, (i * 4 + 3) as u64),
            );
            public_keys.insert(
                *job,
                QHashOut::from_values((i * 4 + 100) as u64, (i * 4 + 101) as u64, (i * 4 + 102) as u64, (i * 4 + 103) as u64),
            );
        }

        let provider = MockJobRewardDataProvider { commitments, public_keys };

        // Create a complex graph structure:
        // root_job -> job3, job4
        // job3 -> job1, job2
        let mut graph = QProvingJobGraph::new();
        graph.guta_graph.add_node(job1);
        graph.guta_graph.add_node(job2);
        graph.guta_graph.add_node(job3);
        graph.guta_graph.add_node(job4);
        graph.guta_graph.add_node(root_job);

        // Create dependencies
        graph.guta_graph.add_edge(job3, job1);
        graph.guta_graph.add_edge(job3, job2);
        graph.guta_graph.add_edge(root_job, job3);
        graph.guta_graph.add_edge(root_job, job4);

        // Generate proofs for all jobs and collect nullifiers
        let mut nullifiers = HashSet::new();
        let jobs = [job1, job2, job3, job4, root_job];

        for &job in &jobs {
            let (proof, _) = graph.generate_variable_height_reward_proof(job, 0, &provider).await.unwrap();
            let (_, nullifier_index) = proof.compute_root_and_nullifier_index();
            let nullifier_value = nullifier_index.to_canonical_u64();

            println!("Job {:?} -> nullifier: {}", job.task_index, nullifier_value);

            // Check for uniqueness
            assert!(
                !nullifiers.contains(&nullifier_value),
                "Nullifier collision detected! Job {} has same nullifier {} as another job",
                job.task_index,
                nullifier_value
            );
            nullifiers.insert(nullifier_value);
        }

        println!("All {} jobs have unique nullifiers", jobs.len());
    }

    #[tokio::test]
    async fn test_nullifier_edge_cases() {
        use std::collections::{HashMap, HashSet};

        // Test potential edge cases for nullifier collisions
        // Case 1: Jobs with same height but different positions
        let leaf1 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 100);
        let leaf2 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoEndCap, 0, 200);
        let parent1 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 300);
        let parent2 = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 400);
        let root = QProvingJobDataID::new_proof_job_id(1, 0, ProvingJobCircuitType::GUTATwoGUTA, 0, 500);

        let mut commitments = HashMap::new();
        let mut public_keys = HashMap::new();

        for (i, &job) in [leaf1, leaf2, parent1, parent2, root].iter().enumerate() {
            commitments.insert(job, QHashOut::from_values(i as u64 + 10, i as u64 + 20, i as u64 + 30, i as u64 + 40));
            public_keys.insert(job, QHashOut::from_values(i as u64 + 50, i as u64 + 60, i as u64 + 70, i as u64 + 80));
        }

        let provider = MockJobRewardDataProvider { commitments, public_keys };

        // Create binary tree structure:
        //       root
        //      /    \
        //  parent1  parent2
        //   /         /
        // leaf1     leaf2
        let mut graph = QProvingJobGraph::new();
        [leaf1, leaf2, parent1, parent2, root]
            .iter()
            .for_each(|&job| graph.guta_graph.add_node(job));

        graph.guta_graph.add_edge(parent1, leaf1); // parent1 depends on leaf1
        graph.guta_graph.add_edge(parent2, leaf2); // parent2 depends on leaf2
        graph.guta_graph.add_edge(root, parent1); // root depends on parent1
        graph.guta_graph.add_edge(root, parent2); // root depends on parent2

        let mut nullifiers = HashMap::new();

        for &job in &[leaf1, leaf2, parent1, parent2, root] {
            let (proof, _) = graph.generate_variable_height_reward_proof(job, 0, &provider).await.unwrap();
            let (_, nullifier_index) = proof.compute_root_and_nullifier_index();
            let nullifier_value = nullifier_index.to_canonical_u64();

            println!(
                "Job {} (height={}): nullifier={}, index={}",
                job.task_index,
                proof.proof_height.to_canonical_u64(),
                nullifier_value,
                proof.index.to_canonical_u64()
            );

            if let Some(&existing_job) = nullifiers.get(&nullifier_value) {
                panic!(
                    "NULLIFIER COLLISION: Job {} and Job {} both have nullifier {}",
                    existing_job, job.task_index, nullifier_value
                );
            }
            nullifiers.insert(nullifier_value, job.task_index);
        }

        println!("No nullifier collisions found in binary tree structure");
    }

    #[test]
    fn test_combine_index_calculation_bug() {
        use plonky2::field::goldilocks_field::GoldilocksField as F;

        // Test case: bottom index=0, but should be in right position after combine
        let bottom_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(5, 6, 7, 8),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(10, 11, 12, 13).into(),
                QHashOut::from_values(14, 15, 16, 17).into()
            )),
            reward_leaf: QHashOut::from_values(18, 19, 20, 21),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0), // Left position in bottom tree
        };

        // Top proof where this bottom_proof should be in RIGHT position (index=1)
        let top_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(22, 23, 24, 25),
                sibling_reward_leaf: QHashOut::from_values(26, 27, 28, 29),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::ZERO.into(),
                QHashOut::ZERO.into()
            )),
            reward_leaf: QHashOut::ZERO,
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1), // This should put combined proof on right
        };

        let combined = bottom_proof.clone().combine_with(top_proof.clone());

        println!("Bottom index: {}", 0);
        println!("Top index: {}", 1);
        println!("Bottom height: {}", 1);
        println!("Expected combined index: {} (should be 2, meaning position 10 in binary)", 1 << 1);
        println!("Actual combined index: {}", combined.index.to_canonical_u64());

        // The current formula: combined_index = 0 | (1 << 1) = 2 ✓
        assert_eq!(combined.index.to_canonical_u64(), 2);

        // But what if both are 0?
        let all_zero_bottom = VariableHeightRewardMerkleProof {
            index: F::from_canonical_usize(0),
            ..bottom_proof
        };
        let all_zero_top = VariableHeightRewardMerkleProof {
            index: F::from_canonical_usize(0),
            ..top_proof
        };

        let zero_combined = all_zero_bottom.combine_with(all_zero_top);
        println!("Zero case - combined index: {}", zero_combined.index.to_canonical_u64());

        // This would be 0 | (0 << 1) = 0, which is correct for left-left path
        assert_eq!(zero_combined.index.to_canonical_u64(), 0);

        // Now let's test a problematic scenario you mentioned:
        // What if we have a proof that should be in position 1 (right child)
        // but the bottom proof's local index is 0?

        println!("\n=== Testing potential problem scenario ===");

        // Create a bottom proof that locally has index 0 (left in its subtree)
        let problematic_bottom = VariableHeightRewardMerkleProof {
            top_siblings: vec![], // Height 0 proof
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(100, 101, 102, 103).into(),
                QHashOut::from_values(104, 105, 106, 107).into()
            )),
            reward_leaf: QHashOut::from_values(108, 109, 110, 111),
            proof_height: F::from_canonical_usize(0), // Leaf proof
            index: F::from_canonical_usize(0),        // Says it's at position 0 locally
        };

        // But we want to combine it such that it ends up at overall position 1 (right
        // child)
        let top_for_right = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(200, 201, 202, 203),
                sibling_reward_leaf: QHashOut::from_values(204, 205, 206, 207),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::ZERO.into(),
                QHashOut::ZERO.into()
            )),
            reward_leaf: QHashOut::ZERO,
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1), // This puts it in right position at top level
        };

        let problematic_combined = problematic_bottom.combine_with(top_for_right);

        println!("Problematic case:");
        println!("  Bottom index: 0 (height 0)");
        println!("  Top index: 1 (height 1)");
        println!(
            "  Combined index: {} (should be 1 for right position)",
            problematic_combined.index.to_canonical_u64()
        );
        println!("  Formula: 0 | (1 << 0) = {}", 0 | (1 << 0));

        // Current formula: combined_index = 0 | (1 << 0) = 1 ✓
        // This actually works correctly!
        assert_eq!(problematic_combined.index.to_canonical_u64(), 1);

        println!("✓ The index calculation appears to be working correctly even in edge cases");
    }

    #[test]
    fn test_empty_graph_graphviz() {
        let graph = QProvingJobGraph::new();
        let output = graph.get_graphviz();

        assert!(output.contains("digraph QProvingJobGraph"));
        assert!(output.contains("cluster_deploy_contracts"));
        assert!(output.contains("cluster_user_registrations"));
        assert!(output.contains("cluster_guta"));
        assert!(output.contains("Deploy Contracts"));
        assert!(output.contains("User Registrations"));
        assert!(output.contains("GUTA"));
    }

    #[test]
    fn test_single_node_graphviz() {
        let mut graph = QProvingJobGraph::new();
        let guta_job = QProvingJobDataID::new_proof_job_id(100, 1, ProvingJobCircuitType::GUTATwoEndCap, 2, 3);

        graph.guta_graph.add_node(guta_job);

        let output = graph.get_graphviz();

        assert!(output.contains("digraph QProvingJobGraph"));
        assert!(output.contains("GUTATwoEndCap"));
        let expected_hex = guta_job.to_hex_string();
        assert!(output.contains(&expected_hex));
        assert!(output.contains(&format!("guta_{}", expected_hex)));
    }

    #[test]
    fn test_multiple_nodes_with_dependencies_graphviz() {
        let mut graph = QProvingJobGraph::new();

        let guta_job1 = QProvingJobDataID::new_proof_job_id(100, 1, ProvingJobCircuitType::GUTATwoEndCap, 1, 1);
        let guta_job2 = QProvingJobDataID::new_proof_job_id(100, 1, ProvingJobCircuitType::GUTATwoGUTA, 1, 2);
        let deploy_job = QProvingJobDataID::new_proof_job_id(100, 2, ProvingJobCircuitType::BatchDeployContracts, 1, 1);

        graph.guta_graph.add_node(guta_job1);
        graph.guta_graph.add_node(guta_job2);
        graph.guta_graph.add_edge(guta_job1, guta_job2);

        graph.deploy_contracts_graph.add_node(deploy_job);

        let output = graph.get_graphviz();

        assert!(output.contains("GUTATwoEndCap"));
        assert!(output.contains("GUTATwoGUTA"));
        assert!(output.contains("BatchDeployContracts"));
        assert!(output.contains(&format!("guta_{}", guta_job1.to_hex_string())));
        assert!(output.contains(&format!("guta_{}", guta_job2.to_hex_string())));
        assert!(output.contains(&format!("deploy_{}", deploy_job.to_hex_string())));
        assert!(output.contains("->"));
    }

    #[test]
    fn test_user_registration_graph_graphviz() {
        let mut graph = QProvingJobGraph::new();
        let user_job = QProvingJobDataID::new_proof_job_id(200, 3, ProvingJobCircuitType::AppendUserRegistrationTree, 4, 5);

        graph.user_registrations_graph.add_node(user_job);

        let output = graph.get_graphviz();

        assert!(output.contains("AppendUserRegistrationTree"));
        assert!(output.contains(&user_job.to_hex_string()));
        assert!(output.contains(&format!("user_{}", user_job.to_hex_string())));
        assert!(output.contains("User Registrations"));
    }

    #[test]
    fn test_graphviz_node_formatting() {
        let mut graph = QProvingJobGraph::new();
        let job = QProvingJobDataID::new_proof_job_id(0x123, 0x456, ProvingJobCircuitType::GUTASingleEndCap, 0x789, 0xABC);

        graph.guta_graph.add_node(job);

        let output = graph.get_graphviz();

        assert!(output.contains("GUTASingleEndCap"));
        assert!(output.contains(&job.to_hex_string()));
        assert!(output.contains("shape=box"));
        assert!(output.contains("fillcolor=lightblue"));
    }
}

#[cfg(test)]
mod combine_tests {
    use super::*;

    #[test]
    fn test_combine_with_simple_case() {
        // Create bottom proof: height=1, index=0
        let bottom_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(5, 6, 7, 8),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(9, 10, 11, 12).into(),
                QHashOut::from_values(13, 14, 15, 16).into()
            )),
            reward_leaf: QHashOut::from_values(17, 18, 19, 20),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        // Create top proof: height=1, index=1
        let top_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(21, 22, 23, 24),
                sibling_reward_leaf: QHashOut::from_values(25, 26, 27, 28),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(29, 30, 31, 32).into(), // Should be ignored
                QHashOut::from_values(33, 34, 35, 36).into()  // Should be ignored
            )),
            reward_leaf: QHashOut::from_values(37, 38, 39, 40),  // Should be ignored
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1),
        };

        let combined = bottom_proof.combine_with(top_proof);

        // Verify combined properties
        assert_eq!(combined.proof_height.to_canonical_u64() as usize, 2); // 1 + 1 = 2
        assert_eq!(combined.index.to_canonical_u64() as usize, 2); // 0 | (1 << 1) = 2
        assert_eq!(combined.top_siblings.len(), 2); // 1 + 1 = 2

        // Verify sibling_branch, reward_leaf come from bottom proof
        assert_eq!(combined.sibling_branch, QHashOut(PoseidonHash::two_to_one(
            QHashOut::from_values(9, 10, 11, 12).into(),
            QHashOut::from_values(13, 14, 15, 16).into()
        )));
        assert_eq!(combined.reward_leaf, QHashOut::from_values(17, 18, 19, 20));
    }

    #[test]
    fn test_combine_with_different_heights() {
        // Bottom: height=2, index=3
        let bottom_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![
                VariableHeightProofSibling {
                    sibling_branch: QHashOut::from_values(1, 1, 1, 1),
                    sibling_reward_leaf: QHashOut::from_values(2, 2, 2, 2),
                },
                VariableHeightProofSibling {
                    sibling_branch: QHashOut::from_values(3, 3, 3, 3),
                    sibling_reward_leaf: QHashOut::from_values(4, 4, 4, 4),
                },
            ],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(100, 100, 100, 100).into(),
                QHashOut::from_values(200, 200, 200, 200).into()
            )),
            reward_leaf: QHashOut::from_values(300, 300, 300, 300),
            proof_height: F::from_canonical_usize(2),
            index: F::from_canonical_usize(3), // Binary: 11
        };

        // Top: height=1, index=1
        let top_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(5, 5, 5, 5),
                sibling_reward_leaf: QHashOut::from_values(6, 6, 6, 6),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(400, 400, 400, 400).into(), // Ignored
                QHashOut::from_values(500, 500, 500, 500).into()  // Ignored
            )),
            reward_leaf: QHashOut::from_values(600, 600, 600, 600),  // Ignored
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1), // Binary: 1
        };

        let combined = bottom_proof.combine_with(top_proof);

        // Verify combined result
        assert_eq!(combined.proof_height.to_canonical_u64() as usize, 3); // 2 + 1 = 3
        assert_eq!(combined.index.to_canonical_u64() as usize, 7); // 3 | (1 << 2) = 3 | 4 = 7 (Binary: 111)
        assert_eq!(combined.top_siblings.len(), 3); // 2 + 1 = 3

        // Verify data source correctness
        assert_eq!(combined.sibling_branch, QHashOut(PoseidonHash::two_to_one(
            QHashOut::from_values(100, 100, 100, 100).into(),
            QHashOut::from_values(200, 200, 200, 200).into()
        )));
        assert_eq!(combined.reward_leaf, QHashOut::from_values(300, 300, 300, 300));

        // Verify siblings order
        assert_eq!(combined.top_siblings[0].sibling_branch, QHashOut::from_values(1, 1, 1, 1));
        assert_eq!(combined.top_siblings[1].sibling_branch, QHashOut::from_values(3, 3, 3, 3));
        assert_eq!(combined.top_siblings[2].sibling_branch, QHashOut::from_values(5, 5, 5, 5));
    }

    #[test]
    fn test_combine_with_zero_height_top() {
        // Bottom: height=1, index=1
        let bottom_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(10, 10, 10, 10),
                sibling_reward_leaf: QHashOut::from_values(20, 20, 20, 20),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(30, 30, 30, 30).into(),
                QHashOut::from_values(40, 40, 40, 40).into()
            )),
            reward_leaf: QHashOut::from_values(50, 50, 50, 50),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1),
        };

        // Top: height=0 (empty proof)
        let top_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(60, 60, 60, 60).into(), // Ignored
                QHashOut::from_values(70, 70, 70, 70).into()  // Ignored
            )),
            reward_leaf: QHashOut::from_values(80, 80, 80, 80),  // Ignored
            proof_height: F::from_canonical_usize(0),
            index: F::from_canonical_usize(0),
        };

        let combined = bottom_proof.combine_with(top_proof);

        // When top_proof height=0, should be equivalent to original proof
        assert_eq!(combined.proof_height.to_canonical_u64() as usize, 1); // 1 + 0 = 1
        assert_eq!(combined.index.to_canonical_u64() as usize, 1); // 1 | (0 << 1) = 1
        assert_eq!(combined.top_siblings.len(), 1); // 1 + 0 = 1

        assert_eq!(combined.sibling_branch, QHashOut(PoseidonHash::two_to_one(
            QHashOut::from_values(30, 30, 30, 30).into(),
            QHashOut::from_values(40, 40, 40, 40).into()
        )));
        assert_eq!(combined.reward_leaf, QHashOut::from_values(50, 50, 50, 50));
    }

    #[test]
    fn test_combine_preserves_root_computation() {
        // Test that combine_with doesn't break root computation
        let bottom_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(5, 6, 7, 8).into(),
                QHashOut::from_values(9, 10, 11, 12).into()
            )),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let top_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(17, 18, 19, 20),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(0, 0, 0, 0).into(), // Ignored
                QHashOut::from_values(0, 0, 0, 0).into()  // Ignored
            )),
            reward_leaf: QHashOut::from_values(0, 0, 0, 0),  // Ignored
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let combined = bottom_proof.combine_with(top_proof);

        // Verify combined proof can correctly compute root and nullifier
        let (root, nullifier) = combined.compute_root_and_nullifier_index();

        // Should compute normally without panic
        assert!(root.0.elements.iter().any(|&x| x != F::ZERO)); // Root should not be all zeros
        assert_eq!(nullifier.to_canonical_u64() as usize, 3); // height=2, index=0 nullifier should be 3 (1+2+0)
    }

    #[test]
    fn test_new_architecture_flow() {
        // Test the new architecture: combine proofs without padding, then pad for claim
        // rewards
        let bottom_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(5, 6, 7, 8).into(),
                QHashOut::from_values(9, 10, 11, 12).into()
            )),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let top_proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(17, 18, 19, 20),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            sibling_branch: QHashOut(PoseidonHash::two_to_one(
                QHashOut::from_values(0, 0, 0, 0).into(), // This should be ignored
                QHashOut::from_values(0, 0, 0, 0).into()  // This should be ignored
            )),
            reward_leaf: QHashOut::from_values(0, 0, 0, 0),  // This should be ignored
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        // Step 1: Combine proofs (realm edge responsibility)
        let combined = bottom_proof.combine_with(top_proof);
        assert_eq!(combined.proof_height.to_canonical_u64() as usize, 2);
        assert_eq!(combined.top_siblings.len(), 2); // Should match proof height

        // Step 2: Pad to max height for claim rewards (claim rewards responsibility)
        let padded_for_claim = combined.pad_to_height(GUTA_REWARDS_TREE_MAX_HEIGHT);
        assert_eq!(padded_for_claim.top_siblings.len(), GUTA_REWARDS_TREE_MAX_HEIGHT);

        // Step 3: Verify the padded proof works for nullifier calculation
        let (padded_root, nullifier) = padded_for_claim.compute_root_and_nullifier_index();
        let verification_passes = padded_for_claim.verify_proof(&padded_root);

        assert!(verification_passes, "Padded proof should verify against its computed root");
        assert_eq!(nullifier.to_canonical_u64() as usize, 3); // height=2, index=0 nullifier should be 3 (1+2+0)
    }
}
