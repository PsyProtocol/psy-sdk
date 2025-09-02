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

use super::{mode::QWorkerMode, traits::QProofStoreAsyncImm};

#[async_trait::async_trait]
pub trait QJobRewardDataProvider {
    async fn get_job_commitment(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>>;
    async fn get_job_worker_public_key(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>>;
}

#[async_trait::async_trait]
impl<T: QProofStoreAsyncImm> QJobRewardDataProvider for T {
    async fn get_job_commitment(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>> {
        let proof = self.get_proof_by_id::<PoseidonGoldilocksConfig, 2>(job_id).await?;
        Ok(QHashOut(HashOut {
            elements: [
                proof.public_inputs[0],
                proof.public_inputs[1],
                proof.public_inputs[2],
                proof.public_inputs[3],
            ],
        }))
    }

    async fn get_job_worker_public_key(&self, job_id: QProvingJobDataID) -> anyhow::Result<QHashOut<F>> {
        let proof = self.get_proof_by_id::<PoseidonGoldilocksConfig, 2>(job_id).await?;
        Ok(QHashOut(HashOut {
            elements: [
                proof.public_inputs[4],
                proof.public_inputs[5],
                proof.public_inputs[6],
                proof.public_inputs[7],
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

pub const GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE: usize = 15;
pub const CONTRACT_DEPLOYMENT_REWARDS_MAX_HEIGHT_MINUS_ONE: usize = 15;
pub const USER_REGISTRATION_REWARDS_MAX_HEIGHT_MINUS_ONE: usize = 15;

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

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
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
    pub top_siblings: Vec<VariableHeightProofSibling>, // MAX_HEIGHT_MINUS_ONE elements
    pub left_branch: QHashOut<F>,
    pub right_branch: QHashOut<F>,
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
            left_branch: self.left_branch,
            right_branch: self.right_branch,
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
            | GUTANoChange => Ok(&self.guta_graph),
            _ => anyhow::bail!("Unsupported circuit type: {:?}", job_id.circuit_type),
        }
    }

    pub async fn generate_variable_height_reward_proof<P: QJobRewardDataProvider>(
        &self,
        job_id: QProvingJobDataID,
        provider: &P,
        max_height: usize,
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

        if path_to_root.len() > max_height {
            anyhow::bail!("Path to root exceeds max_height: {} > {}", path_to_root.len(), max_height);
        }

        let actual_height = path_to_root.len();
        let mut top_siblings = Vec::new();

        for &(child, parent) in path_to_root.iter() {
            if let Some(parent_dependencies) = graph.get_dependencies(&parent) {
                let deps_vec: Vec<_> = parent_dependencies.iter().cloned().collect();

                let (sibling_branch, sibling_reward_leaf) = if deps_vec.len() == 2 {
                    let sibling_id = if child == deps_vec[0] { deps_vec[1] } else { deps_vec[0] };
                    let sibling_branch = provider.get_job_commitment(sibling_id).await?;
                    let sibling_reward_leaf = provider.get_job_worker_public_key(sibling_id).await?;
                    (sibling_branch, sibling_reward_leaf)
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
            } else {
                top_siblings.push(VariableHeightProofSibling {
                    sibling_branch: QHashOut(HashOut { elements: [F::ZERO; 4] }),
                    sibling_reward_leaf: QHashOut(HashOut { elements: [F::ZERO; 4] }),
                });
            }
        }

        while top_siblings.len() < max_height {
            top_siblings.push(VariableHeightProofSibling {
                sibling_branch: QHashOut(HashOut { elements: [F::ZERO; 4] }),
                sibling_reward_leaf: QHashOut(HashOut { elements: [F::ZERO; 4] }),
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

        let (left_branch, right_branch) = if let Some(job_dependencies) = graph.get_dependencies(&job_id) {
            let deps_vec: Vec<_> = job_dependencies.iter().cloned().collect();
            if deps_vec.len() == 2 {
                let left = provider.get_job_commitment(deps_vec[0]).await?;
                let right = provider.get_job_commitment(deps_vec[1]).await?;
                (left, right)
            } else if deps_vec.len() == 1 {
                let left = provider.get_job_commitment(deps_vec[0]).await?;
                let right = QHashOut(HashOut { elements: [F::ZERO; 4] });
                (left, right)
            } else {
                let zero = QHashOut(HashOut { elements: [F::ZERO; 4] });
                (zero, zero)
            }
        } else {
            let zero = QHashOut(HashOut { elements: [F::ZERO; 4] });
            (zero, zero)
        };

        let reward_leaf = provider.get_job_worker_public_key(job_id).await?;

        let proof = VariableHeightRewardMerkleProof {
            top_siblings,
            left_branch,
            right_branch,
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

                        output.push_str(&format!(
                            "    \"{}\" -> \"{}\";\n",
                            from_node, to_node
                        ));
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
            group_id: 0,
            circuit_type: ProvingJobCircuitType::Unknown,
            sub_group_id: realm_id,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn notify_block_complete(checkpoint_id: u64) -> Self {
        Self {
            topic: QJobTopic::NotifyCoordinatorComplete,
            goal_id: checkpoint_id,
            group_id: 0,
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
    pub fn guta_two_end_cap_witness(checkpoint_id: u64, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            ProvingJobCircuitType::GUTATwoEndCap.to_circuit_group_id(),
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTATwoEndCap,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_two_agg_witness(checkpoint_id: u64, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            ProvingJobCircuitType::GUTATwoGUTA.to_circuit_group_id(),
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTATwoGUTA,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_left_end_cap_right_guta_witness(checkpoint_id: u64, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA.to_circuit_group_id(),
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_left_guta_right_end_cap_witness(checkpoint_id: u64, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap.to_circuit_group_id(),
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTALeftGUTARightEndCap,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn guta_single_end_cap_witness(checkpoint_id: u64, sub_group_id: u32, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            ProvingJobCircuitType::GUTASingleEndCap.to_circuit_group_id(),
            sub_group_id,
            task_index,
            ProvingJobCircuitType::GUTASingleEndCap,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn core_op_witness(circuit_type: ProvingJobCircuitType, checkpoint_id: u64, task_index: u32) -> Self {
        Self::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            circuit_type.to_circuit_group_id(),
            0,
            task_index,
            circuit_type,
            ProvingJobDataType::InputWitness,
            0,
        )
    }
    pub fn transfer_signature_proof(rpc_node_id: u32, block_id: u64, transfer_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: block_id,
            group_id: 1,
            circuit_type: ProvingJobCircuitType::WrappedSignatureProof,
            sub_group_id: rpc_node_id,
            task_index: transfer_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn end_cap_proof(rpc_node_id: u32, checkpoint_id: u64, user_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: checkpoint_id,
            group_id: 1,
            circuit_type: ProvingJobCircuitType::UserEndCap,
            sub_group_id: rpc_node_id,
            task_index: user_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn withdrawal_signature_proof(rpc_node_id: u32, block_id: u64, withdrawal_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: block_id,
            group_id: 2,
            circuit_type: ProvingJobCircuitType::WrappedSignatureProof,
            sub_group_id: rpc_node_id,
            task_index: withdrawal_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn claim_deposit_l1_signature_proof(rpc_node_id: u32, block_id: u64, deposit_id: u32) -> Self {
        Self {
            topic: QJobTopic::BlockUserSignatureProof,
            goal_id: block_id,
            group_id: 3,
            circuit_type: ProvingJobCircuitType::Secp256K1SignatureProof,
            sub_group_id: rpc_node_id,
            task_index: deposit_id,
            data_type: ProvingJobDataType::BaseInputProof,
            data_index: 0,
        }
    }
    pub fn new_proof_job_id(goal_id: u64, circuit_type: ProvingJobCircuitType, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id,
            circuit_type,
            group_id,
            sub_group_id,
            task_index,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn new_groth16_proof_job_id(goal_id: u64, circuit_type: ProvingJobCircuitType, group_id: u32, sub_group_id: u32, task_index: u32) -> Self {
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
    pub fn get_block_aggregate_jobs_group(block_id: u64, group_id: u32, task_index: u32) -> Self {
        Self {
            topic: QJobTopic::AggregateJobs,
            goal_id: block_id,
            group_id,
            circuit_type: ProvingJobCircuitType::Unknown,
            sub_group_id: 0,
            task_index,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn block_agg_state_part_1_input_witness(block_id: u64) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: block_id,
            group_id: ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA.to_circuit_group_id(),
            circuit_type: ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn block_agg_state_part_2_input_witness(block_id: u64) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: block_id,
            group_id: ProvingJobCircuitType::AggAddProcessL1WithdrawalAddL1Deposit.to_circuit_group_id(),
            circuit_type: ProvingJobCircuitType::AggAddProcessL1WithdrawalAddL1Deposit,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn block_state_transition_input_witness(block_id: u64) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: block_id,
            group_id: ProvingJobCircuitType::GenerateRollupStateTransitionProof.to_circuit_group_id(),
            circuit_type: ProvingJobCircuitType::GenerateRollupStateTransitionProof,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn sighash_introspection_input_witness(block_id: u64, input_id: usize) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: block_id,
            group_id: ProvingJobCircuitType::GenerateSigHashIntrospectionProof.to_circuit_group_id(),
            circuit_type: ProvingJobCircuitType::GenerateSigHashIntrospectionProof,
            sub_group_id: 0,
            task_index: input_id as u32,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn sighash_final_input_witness(block_id: u64, input_id: usize) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: block_id,
            group_id: ProvingJobCircuitType::GenerateFinalSigHashProof.to_circuit_group_id(),
            circuit_type: ProvingJobCircuitType::GenerateFinalSigHashProof,
            sub_group_id: input_id as u32,
            task_index: input_id as u32,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn wrap_sighash_final_bls3812_input_witness(block_id: u64, input_id: usize) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: block_id,
            group_id: ProvingJobCircuitType::WrapFinalSigHashProofBLS12381.to_circuit_group_id(),
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
    pub fn compute_root_and_nullifier_index(&self, max_height_minus_one: usize) -> (QHashOut<F>, F) {
        let proof_height = self.proof_height.to_canonical_u64() as usize;
        let index = self.index.to_canonical_u64();

        assert!(proof_height <= max_height_minus_one);
        assert!(index < (1 << max_height_minus_one));

        let mut nullifier_base = 0u64;
        let mut nullifier_level_start_index_multiplier = 1u64;

        let mut current_node_value = PoseidonHash::two_to_one(PoseidonHash::two_to_one(self.left_branch.0, self.right_branch.0), self.reward_leaf.0);

        for i in 0..max_height_minus_one {
            if i < proof_height && i < self.top_siblings.len() {
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
            } else if i < self.top_siblings.len() {
                let sibling = &self.top_siblings[i];
                assert!(sibling.sibling_reward_leaf.0.elements.iter().all(|&x| x == F::ZERO));
                assert!(sibling.sibling_branch.0.elements.iter().all(|&x| x == F::ZERO));
                assert!((index >> i) & 1 == 0);
            }
        }

        let nullifier_final_index = F::from_canonical_u64(nullifier_base + index);
        (QHashOut(current_node_value), nullifier_final_index)
    }

    pub fn verify_proof(&self, expected_root: &QHashOut<F>, max_height_minus_one: usize) -> bool {
        let (computed_root, _) = self.compute_root_and_nullifier_index(max_height_minus_one);
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
            left_branch: QHashOut::from_values(5, 6, 7, 8),
            right_branch: QHashOut::from_values(9, 10, 11, 12),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let (root, nullifier) = proof.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);

        assert!(proof.verify_proof(&root, GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE));
        assert!(nullifier != F::ZERO);
    }

    #[test]
    fn test_job_graph_creation() {
        let graph = QProvingJobGraph::new();
        assert!(graph
            .deploy_contracts_graph
            .get_dependencies(&QProvingJobDataID::new_proof_job_id(
                1,
                ProvingJobCircuitType::BatchDeployContracts,
                0,
                0,
                0
            ))
            .is_none());
    }

    #[test]
    fn test_get_graph_for_job_selection() {
        let graph = QProvingJobGraph::new();

        let deploy_job = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::BatchDeployContracts, 0, 0, 0);
        let user_reg_job = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 0, 0);
        let guta_job = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTAOnlyRegisterUsers, 0, 0, 0);

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
            left_branch: QHashOut::from_values(17, 18, 19, 20),
            right_branch: QHashOut::from_values(21, 22, 23, 24),
            reward_leaf: QHashOut::from_values(25, 26, 27, 28),
            proof_height: F::from_canonical_usize(2),
            index: F::from_canonical_usize(1),
        };

        let (root, nullifier) = proof.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);

        assert!(proof.verify_proof(&root, GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE));
        assert!(nullifier != F::ZERO);
    }

    #[test]
    fn test_variable_height_proof_zero_height() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![],
            left_branch: QHashOut::from_values(1, 2, 3, 4),
            right_branch: QHashOut::from_values(5, 6, 7, 8),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(0),
            index: F::from_canonical_usize(0),
        };

        let (root, nullifier) = proof.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);

        assert!(proof.verify_proof(&root, GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE));
        assert_eq!(nullifier, F::from_canonical_usize(0));
    }

    #[test]
    fn test_compute_root_from_variable_height_proof() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(100, 101, 102, 103),
                sibling_reward_leaf: QHashOut::from_values(200, 201, 202, 203),
            }],
            left_branch: QHashOut::from_values(1, 2, 3, 4),
            right_branch: QHashOut::from_values(5, 6, 7, 8),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let root1 = compute_root_from_variable_height_proof(&proof);
        let (root2, _) = proof.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);

        assert_eq!(root1, root2);
    }

    #[test]
    fn test_job_graph_all_circuit_types() {
        let graph = QProvingJobGraph::new();

        let deploy_jobs = vec![
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::BatchDeployContracts, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::BatchDeployContractsAggregate, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::DummyBatchDeployContractsAggregate, 0, 0, 0),
        ];

        let user_reg_jobs = vec![
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTreeAggregate, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate, 0, 0, 0),
        ];

        let guta_jobs = vec![
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTAOnlyRegisterUsers, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTARegisterUsers, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTATwoEndCap, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTATwoGUTA, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTALeftEndCapRightGUTA, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTALeftGUTARightEndCap, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTASingleEndCap, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTAVerifyToCap, 0, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::GUTANoChange, 0, 0, 0),
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
            left_branch: QHashOut::from_values(5, 6, 7, 8),
            right_branch: QHashOut::from_values(9, 10, 11, 12),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(0),
        };

        let proof2 = VariableHeightRewardMerkleProof {
            top_siblings: vec![VariableHeightProofSibling {
                sibling_branch: QHashOut::from_values(1, 2, 3, 4),
                sibling_reward_leaf: QHashOut::from_values(0, 0, 0, 0),
            }],
            left_branch: QHashOut::from_values(5, 6, 7, 8),
            right_branch: QHashOut::from_values(9, 10, 11, 12),
            reward_leaf: QHashOut::from_values(13, 14, 15, 16),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1),
        };

        let (_, nullifier1) = proof1.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);
        let (_, nullifier2) = proof2.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);

        assert_ne!(nullifier1, nullifier2);
        assert_eq!(nullifier1, F::from_canonical_usize(1));
        assert_eq!(nullifier2, F::from_canonical_usize(2));
    }

    #[test]
    fn test_job_data_id_serialization() {
        let job_id = QProvingJobDataID::new_proof_job_id(12345, ProvingJobCircuitType::BatchDeployContracts, 100, 200, 300);

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
            ProvingJobCircuitType::GUTAOnlyRegisterUsers,
            0x12345678,
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
        assert_eq!(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE, 15);
        assert_eq!(CONTRACT_DEPLOYMENT_REWARDS_MAX_HEIGHT_MINUS_ONE, 15);
        assert_eq!(USER_REGISTRATION_REWARDS_MAX_HEIGHT_MINUS_ONE, 15);

        let max_index = (1u64 << GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE) - 1;
        assert_eq!(max_index, 32767); // 2^15 - 1
    }

    #[test]
    fn test_proof_height_validation() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![],
            left_branch: QHashOut::from_values(1, 2, 3, 4),
            right_branch: QHashOut::from_values(5, 6, 7, 8),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(20), // > max_height_minus_one
            index: F::from_canonical_usize(0),
        };

        std::panic::catch_unwind(|| {
            proof.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);
        })
        .expect_err("Should panic when proof_height > max_height_minus_one");
    }

    #[test]
    fn test_index_validation() {
        let proof = VariableHeightRewardMerkleProof {
            top_siblings: vec![],
            left_branch: QHashOut::from_values(1, 2, 3, 4),
            right_branch: QHashOut::from_values(5, 6, 7, 8),
            reward_leaf: QHashOut::from_values(9, 10, 11, 12),
            proof_height: F::from_canonical_usize(1),
            index: F::from_canonical_usize(1 << 16), // > 2^max_height_minus_one
        };

        std::panic::catch_unwind(|| {
            proof.compute_root_and_nullifier_index(GUTA_REWARDS_TREE_MAX_HEIGHT_MINUS_ONE);
        })
        .expect_err("Should panic when index >= 2^max_height_minus_one");
    }

    #[test]
    fn test_job_reward_data_provider_trait() {
        use std::collections::HashMap;

        let job_a = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 0, 100);

        let mut commitments = HashMap::new();
        let mut public_keys = HashMap::new();

        commitments.insert(job_a, QHashOut::from_values(10, 11, 12, 13));
        public_keys.insert(job_a, QHashOut::from_values(20, 21, 22, 23));

        let _provider = MockJobRewardDataProvider { commitments, public_keys };
    }

    #[test]
    fn test_job_graph_dependency_structure() {
        let job_a = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 0, 100);
        let job_b = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTreeAggregate, 0, 0, 200);

        let mut graph = QProvingJobGraph::new();
        graph.user_registrations_graph.add_node(job_a);
        graph.user_registrations_graph.add_node(job_b);
        graph.user_registrations_graph.add_edge(job_b, job_a);

        assert!(graph.user_registrations_graph.get_dependents(&job_a).unwrap().contains(&job_b));
        assert!(graph.user_registrations_graph.get_dependencies(&job_b).unwrap().contains(&job_a));

        let job_c = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTree, 0, 0, 300);
        let job_d = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AppendUserRegistrationTreeAggregate, 0, 0, 400);

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
        let guta_job = QProvingJobDataID::new_proof_job_id(100, ProvingJobCircuitType::GUTATwoEndCap, 1, 2, 3);

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

        let guta_job1 = QProvingJobDataID::new_proof_job_id(100, ProvingJobCircuitType::GUTATwoEndCap, 1, 1, 1);
        let guta_job2 = QProvingJobDataID::new_proof_job_id(100, ProvingJobCircuitType::GUTATwoGUTA, 1, 1, 2);
        let deploy_job = QProvingJobDataID::new_proof_job_id(100, ProvingJobCircuitType::BatchDeployContracts, 2, 1, 1);

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
        let user_job = QProvingJobDataID::new_proof_job_id(200, ProvingJobCircuitType::AppendUserRegistrationTree, 3, 4, 5);

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
        let job = QProvingJobDataID::new_proof_job_id(0x123, ProvingJobCircuitType::GUTASingleEndCap, 0x456, 0x789, 0xABC);

        graph.guta_graph.add_node(job);

        let output = graph.get_graphviz();

        assert!(output.contains("GUTASingleEndCap"));
        assert!(output.contains(&job.to_hex_string()));
        assert!(output.contains("shape=box"));
        assert!(output.contains("fillcolor=lightblue"));
    }
}

pub fn compute_root_from_variable_height_proof(proof: &VariableHeightRewardMerkleProof) -> QHashOut<F> {
    let mut current_node_value = PoseidonHash::two_to_one(proof.left_branch.0, proof.right_branch.0);
    current_node_value = PoseidonHash::two_to_one(current_node_value, proof.reward_leaf.0);

    let proof_height = proof.proof_height.to_canonical_u64() as usize;
    let index = proof.index.to_canonical_u64();

    for i in 0..proof_height.min(proof.top_siblings.len()) {
        let sibling = &proof.top_siblings[i];
        let sibling_node = sibling.sibling_branch.0;
        let reward_leaf = sibling.sibling_reward_leaf.0;

        let index_bit = (index >> i) & 1;
        let branch_path_hash = if index_bit == 0 {
            PoseidonHash::two_to_one(current_node_value, sibling_node)
        } else {
            PoseidonHash::two_to_one(sibling_node, current_node_value)
        };
        current_node_value = PoseidonHash::two_to_one(branch_path_hash, reward_leaf);
    }

    QHashOut(current_node_value)
}
