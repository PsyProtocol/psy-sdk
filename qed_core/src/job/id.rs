use super::mode::QWorkerMode;
use crate::config::network_constants::{QED_CHECKPOINT_JOB_ID_CHANNEL, REALM_PROOF_SYNC_CHANNEL};
use crate::data::qhashout::QHashOut;
use crate::job::drain_queue::{DrainQueueMetadata, DrainQueueMetadataTagged};
use crate::job::history_queue::{HistoryQueueMetadata, HistoryQueueMetadataTagged};
use hex::FromHexError;
use indexmap::{IndexMap, IndexSet};
use kvq::traits::KVQSerializable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::HashOut;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::{Hasher, PoseidonGoldilocksConfig};
use plonky2::plonk::proof::ProofWithPublicInputs;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::serde_as;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

type F = GoldilocksField;

#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
#[repr(u8)]
pub enum QCircuitCommonGatesType {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
}
#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
#[repr(u8)]
pub enum QJobTopic {
    GenerateStandardProof = 0,
    GenerateGroth16Proof = 1,
    BlockUserSignatureProof = 2,
    NotifyOrchestratorComplete = 3,
    AggregateJobs = 4,
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
            3 => Ok(QJobTopic::NotifyOrchestratorComplete),
            4 => Ok(QJobTopic::AggregateJobs),
            _ => Err(anyhow::format_err!("Invalid QJobTopic value: {}", value)),
        }
    }
}

#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
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
            _ => Err(anyhow::format_err!(
                "Invalid ProvingJobDataType value: {}",
                value
            )),
        }
    }
}
impl From<ProvingJobDataType> for u8 {
    fn from(value: ProvingJobDataType) -> u8 {
        value as u8
    }
}

#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
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
            ProvingJobCircuitType::AppendUserRegistrationTree => {
                ProvingJobCircuitType::AppendUserRegistrationTree
            }
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => {
                ProvingJobCircuitType::AppendUserRegistrationTree
            }
            ProvingJobCircuitType::AddL1Deposit => ProvingJobCircuitType::AddL1Deposit,
            ProvingJobCircuitType::AddL1DepositAggregate => ProvingJobCircuitType::AddL1Deposit,
            ProvingJobCircuitType::ClaimL1Deposit => ProvingJobCircuitType::ClaimL1Deposit,
            ProvingJobCircuitType::ClaimL1DepositAggregate => ProvingJobCircuitType::ClaimL1Deposit,
            ProvingJobCircuitType::AddL1Withdrawal => ProvingJobCircuitType::AddL1Withdrawal,
            ProvingJobCircuitType::AddL1WithdrawalAggregate => {
                ProvingJobCircuitType::AddL1Withdrawal
            }
            ProvingJobCircuitType::BatchDeployContracts => {
                ProvingJobCircuitType::BatchDeployContracts
            }
            ProvingJobCircuitType::BatchDeployContractsAggregate => {
                ProvingJobCircuitType::BatchDeployContracts
            }
            ProvingJobCircuitType::ProcessL1Withdrawal => {
                ProvingJobCircuitType::ProcessL1Withdrawal
            }
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => {
                ProvingJobCircuitType::ProcessL1Withdrawal
            }
            _ => anyhow::bail!("circuit type {:?} does not have a leaf type", self),
        };
        Ok(leaf_type)
    }

    pub fn get_agg_circuit_type_or_err(&self) -> anyhow::Result<Self> {
        let leaf_type = match self {
            ProvingJobCircuitType::AppendUserRegistrationTree => {
                ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => {
                ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::AddL1Deposit => ProvingJobCircuitType::AddL1DepositAggregate,
            ProvingJobCircuitType::AddL1DepositAggregate => {
                ProvingJobCircuitType::AddL1DepositAggregate
            }
            ProvingJobCircuitType::ClaimL1Deposit => ProvingJobCircuitType::ClaimL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1DepositAggregate => {
                ProvingJobCircuitType::ClaimL1DepositAggregate
            }
            ProvingJobCircuitType::AddL1Withdrawal => {
                ProvingJobCircuitType::AddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::AddL1WithdrawalAggregate => {
                ProvingJobCircuitType::AddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::BatchDeployContracts => {
                ProvingJobCircuitType::BatchDeployContractsAggregate
            }
            ProvingJobCircuitType::BatchDeployContractsAggregate => {
                ProvingJobCircuitType::BatchDeployContractsAggregate
            }
            ProvingJobCircuitType::ProcessL1Withdrawal => {
                ProvingJobCircuitType::ProcessL1WithdrawalAggregate
            }
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => {
                ProvingJobCircuitType::ProcessL1WithdrawalAggregate
            }
            _ => anyhow::bail!(
                "circuit type {:?} does not have a aggregated circuit type",
                self
            ),
        };
        Ok(leaf_type)
    }

    pub fn get_agg_dummy_circuit_type_or_err(&self) -> anyhow::Result<Self> {
        let leaf_type = match self {
            ProvingJobCircuitType::AppendUserRegistrationTree => {
                ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => {
                ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => {
                ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::AddL1Deposit => {
                ProvingJobCircuitType::DummyAddL1DepositAggregate
            }
            ProvingJobCircuitType::AddL1DepositAggregate => {
                ProvingJobCircuitType::DummyAddL1DepositAggregate
            }
            ProvingJobCircuitType::DummyAddL1DepositAggregate => {
                ProvingJobCircuitType::DummyAddL1DepositAggregate
            }
            ProvingJobCircuitType::ClaimL1Deposit => {
                ProvingJobCircuitType::DummyClaimL1DepositAggregate
            }
            ProvingJobCircuitType::ClaimL1DepositAggregate => {
                ProvingJobCircuitType::DummyClaimL1DepositAggregate
            }
            ProvingJobCircuitType::DummyClaimL1DepositAggregate => {
                ProvingJobCircuitType::DummyClaimL1DepositAggregate
            }
            ProvingJobCircuitType::AddL1Withdrawal => {
                ProvingJobCircuitType::DummyAddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::AddL1WithdrawalAggregate => {
                ProvingJobCircuitType::DummyAddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::DummyAddL1WithdrawalAggregate => {
                ProvingJobCircuitType::DummyAddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::BatchDeployContracts => {
                ProvingJobCircuitType::DummyBatchDeployContractsAggregate
            }
            ProvingJobCircuitType::BatchDeployContractsAggregate => {
                ProvingJobCircuitType::DummyBatchDeployContractsAggregate
            }
            ProvingJobCircuitType::DummyBatchDeployContractsAggregate => {
                ProvingJobCircuitType::DummyBatchDeployContractsAggregate
            }
            ProvingJobCircuitType::ProcessL1Withdrawal => {
                ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate
            }
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => {
                ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate
            }
            ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate => {
                ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate
            }
            _ => anyhow::bail!(
                "circuit type {:?} does not have a aggregated dummy circuit type",
                self
            ),
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
            255 => Ok(ProvingJobCircuitType::Unknown),
            _ => Err(anyhow::format_err!(
                "Invalid ProvingJobCircuitType value: {}",
                value
            )),
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
pub struct QProvingJobDataIDSerializedWrapped(
    #[serde_as(as = "serde_with::hex::Hex")] pub QProvingJobDataIDSerialized,
);

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JobsTask {
    pub task_id: TaskId,
    pub job_ids: Vec<QProvingJobDataID>,
}

impl JobsTask {
    pub fn new(job_ids: &[QProvingJobDataID]) -> Self {
        let task_id = TaskId::new_debug();
        Self {
            task_id,
            job_ids: job_ids.to_vec(),
        }
    }

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }
}
static DEBUG_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn new_debug() -> Self {
        let counter = DEBUG_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&counter.to_le_bytes());
        TaskId(Uuid::from_bytes(bytes))
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Color {
    White,
    Grey,
    Black,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobProofSibling {
    pub hash: QHashOut<F>,
    pub is_left: bool,
    pub parent_public_key: Option<QHashOut<F>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobProof {
    pub job_id: QProvingJobDataID,
    pub value: QHashOut<F>,
    pub siblings: Vec<JobProofSibling>,
    pub root: QHashOut<F>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobsTaskGraph {
    pub tasks: IndexMap<TaskId, JobsTask>,
    pub deps: IndexMap<TaskId, IndexSet<TaskId>>,
    pub deps_on: IndexMap<TaskId, IndexSet<TaskId>>,
}

impl JobsTaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: IndexMap::new(),
            deps: IndexMap::new(),
            deps_on: IndexMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.tasks.clear();
        self.deps.clear();
        self.deps_on.clear();
    }

    pub fn add_task(&mut self, task: JobsTask) {
        let task_id = task.task_id();
        self.tasks.insert(task_id, task);
    }

    pub fn add_dep(&mut self, task: JobsTask, dep_task: JobsTask) {
        self.deps
            .entry(task.task_id())
            .or_default()
            .insert(dep_task.task_id());
        self.deps_on
            .entry(dep_task.task_id())
            .or_default()
            .insert(task.task_id());
        self.add_task(task);
        self.add_task(dep_task);
    }

    pub fn ts_inner(
        &self,
        task: TaskId,
        colors: &mut IndexMap<TaskId, Color>,
        visitor: &mut impl FnMut(TaskId),
    ) {
        colors.insert(task, Color::Grey);
        if let Some(deps) = self.deps.get(&task) {
            for &dep in deps {
                match colors.get(&dep) {
                    Some(Color::Grey) => panic!("cycle detected"),
                    Some(Color::Black) => {
                        return;
                    }
                    None => self.ts_inner(dep, colors, visitor),
                    _ => {}
                }
            }
        }
        visitor(task);
        colors.insert(task, Color::Black);
    }

    pub fn ts(&self) -> Vec<TaskId> {
        let mut sorted = Vec::new();
        let mut colors = IndexMap::new();

        for &task_id in self.tasks.keys() {
            if !colors.contains_key(&task_id) {
                self.ts_inner(task_id, &mut colors, &mut |task| sorted.push(task));
            }
        }
        sorted
    }

    pub fn ts_task(&self) -> Vec<&JobsTask> {
        self.ts()
            .into_iter()
            .filter_map(|task_id| self.tasks.get(&task_id))
            .collect()
    }

    pub fn get_task(&self, task_id: TaskId) -> Option<&JobsTask> {
        self.tasks.get(&task_id)
    }

    pub fn generate_proof<PS: crate::job::traits::QProofStore>(
        &self,
        leaf_job_id: QProvingJobDataID,
        proof_store: &PS,
    ) -> anyhow::Result<JobProof> {
        let mut leaf_task_id = None;
        let mut leaf_job_index = 0;

        for (task_id, task) in &self.tasks {
            if let Some(idx) = task.job_ids.iter().position(|&id| id == leaf_job_id) {
                leaf_task_id = Some(*task_id);
                leaf_job_index = idx;
                break;
            }
        }

        let leaf_task_id =
            leaf_task_id.ok_or_else(|| anyhow::anyhow!("Leaf job not found in any task"))?;

        let task_levels = self.get_task_levels();

        let mut current_level = 0;
        for (level_idx, level_tasks) in task_levels.iter().enumerate() {
            if level_tasks.contains(&leaf_task_id) {
                current_level = level_idx;
                break;
            }
        }

        let leaf_proof: ProofWithPublicInputs<F, PoseidonGoldilocksConfig, 2> =
            proof_store.get_proof_by_id(leaf_job_id.get_output_id())?;

        let leaf_value = if leaf_proof.public_inputs.len() >= 4 {
            let mut elements = [F::ZERO; 4];
            elements.copy_from_slice(&leaf_proof.public_inputs[0..4]);
            QHashOut(HashOut { elements })
        } else {
            return Err(anyhow::anyhow!("Invalid proof public inputs length"));
        };

        let mut siblings = Vec::new();
        let mut current_task_id = leaf_task_id;
        let mut current_job_index = leaf_job_index;

        while current_level < task_levels.len() - 1 {
            let current_task = &self.tasks[&current_task_id];

            let sibling_index = if current_job_index % 2 == 0 {
                if current_job_index + 1 < current_task.job_ids.len() {
                    Some(current_job_index + 1)
                } else {
                    None
                }
            } else {
                Some(current_job_index - 1)
            };

            if let Some(sibling_idx) = sibling_index {
                let sibling_job_id = current_task.job_ids[sibling_idx];
                let sibling_proof: ProofWithPublicInputs<F, PoseidonGoldilocksConfig, 2> =
                    proof_store.get_proof_by_id(sibling_job_id.get_output_id())?;

                let sibling_commitment = if sibling_proof.public_inputs.len() >= 4 {
                    let mut elements = [F::ZERO; 4];
                    elements.copy_from_slice(&sibling_proof.public_inputs[0..4]);
                    QHashOut(HashOut { elements })
                } else {
                    return Err(anyhow::anyhow!(
                        "Invalid sibling proof public inputs length"
                    ));
                };

                siblings.push(JobProofSibling {
                    hash: sibling_commitment,
                    is_left: sibling_idx < current_job_index,
                    parent_public_key: None, // Will be filled when we move to parent
                });
            }

            if let Some(parent_tasks) = self.deps_on.get(&current_task_id) {
                if let Some(&parent_task_id) = parent_tasks.iter().next() {
                    current_job_index = current_job_index / 2;

                    if sibling_index.is_some() {
                        let parent_job_id = self.tasks[&parent_task_id].job_ids[current_job_index];
                        let parent_proof: ProofWithPublicInputs<F, PoseidonGoldilocksConfig, 2> =
                            proof_store.get_proof_by_id(parent_job_id.get_output_id())?;

                        if parent_proof.public_inputs.len() >= 8 {
                            let mut elements = [F::ZERO; 4];
                            elements.copy_from_slice(&parent_proof.public_inputs[4..8]);
                            let parent_public_key = QHashOut(HashOut { elements });

                            if let Some(last_sibling) = siblings.last_mut() {
                                last_sibling.parent_public_key = Some(parent_public_key);
                            }
                        }
                    }

                    current_task_id = parent_task_id;
                    current_level += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let root = self.compute_root_from_proof(&leaf_value, &siblings);

        Ok(JobProof {
            job_id: leaf_job_id,
            value: leaf_value,
            siblings,
            root,
        })
    }

    pub fn generate_proofs_batch<PS: crate::job::traits::QProofStore>(
        &self,
        leaf_job_ids: &[QProvingJobDataID],
        proof_store: &PS,
    ) -> anyhow::Result<Vec<JobProof>> {
        let mut results = Vec::new();

        for &leaf_job_id in leaf_job_ids {
            let proof = self.generate_proof(leaf_job_id, proof_store)?;
            results.push(proof);
        }

        Ok(results)
    }

    fn get_task_levels(&self) -> Vec<Vec<TaskId>> {
        let mut levels = Vec::new();
        let mut processed = IndexSet::new();

        let mut current_level = Vec::new();
        for &task_id in self.tasks.keys() {
            if !self.deps.contains_key(&task_id) || self.deps[&task_id].is_empty() {
                current_level.push(task_id);
                processed.insert(task_id);
            }
        }

        if current_level.is_empty() {
            return levels;
        }

        levels.push(current_level.clone());

        while !current_level.is_empty() {
            let mut next_level = Vec::new();

            for &task_id in &current_level {
                if let Some(parent_tasks) = self.deps_on.get(&task_id) {
                    for &parent_task_id in parent_tasks {
                        if let Some(parent_deps) = self.deps.get(&parent_task_id) {
                            if parent_deps.iter().all(|dep| processed.contains(dep)) {
                                if !processed.contains(&parent_task_id) {
                                    next_level.push(parent_task_id);
                                    processed.insert(parent_task_id);
                                }
                            }
                        }
                    }
                }
            }

            if !next_level.is_empty() {
                levels.push(next_level.clone());
            }
            current_level = next_level;
        }

        levels
    }

    pub fn verify_proof(&self, proof: &JobProof) -> bool {
        let computed_root = self.compute_root_from_proof(&proof.value, &proof.siblings);
        computed_root == proof.root
    }

    fn compute_root_from_proof(
        &self,
        value: &QHashOut<F>,
        siblings: &[JobProofSibling],
    ) -> QHashOut<F> {
        let mut current_hash = *value;

        for sibling in siblings {
            current_hash = if sibling.is_left {
                QHashOut(PoseidonHash::two_to_one(sibling.hash.0, current_hash.0))
            } else {
                QHashOut(PoseidonHash::two_to_one(current_hash.0, sibling.hash.0))
            };

            if let Some(parent_public_key) = sibling.parent_public_key {
                current_hash = QHashOut(PoseidonHash::two_to_one(
                    current_hash.0,
                    parent_public_key.0,
                ));
            }
        }

        current_hash
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
    pub fn new_notify_realm_complete_witness(checkpoint_id: u64, realm_id: u32) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: checkpoint_id,
            circuit_type: ProvingJobCircuitType::NotifyRealmComplete,
            group_id: ProvingJobCircuitType::NotifyRealmComplete.to_circuit_group_id(),
            sub_group_id: realm_id,
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
    pub fn guta_two_end_cap_witness(
        checkpoint_id: u64,
        sub_group_id: u32,
        task_index: u32,
    ) -> Self {
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
    pub fn guta_left_end_cap_right_guta_witness(
        checkpoint_id: u64,
        sub_group_id: u32,
        task_index: u32,
    ) -> Self {
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
    pub fn guta_left_guta_right_end_cap_witness(
        checkpoint_id: u64,
        sub_group_id: u32,
        task_index: u32,
    ) -> Self {
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
    pub fn guta_single_end_cap_witness(
        checkpoint_id: u64,
        sub_group_id: u32,
        task_index: u32,
    ) -> Self {
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
    pub fn core_op_witness(
        circuit_type: ProvingJobCircuitType,
        checkpoint_id: u64,
        task_index: u32,
    ) -> Self {
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
    pub fn claim_deposit_l1_signature_proof(
        rpc_node_id: u32,
        block_id: u64,
        deposit_id: u32,
    ) -> Self {
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
    pub fn new_proof_job_id(
        goal_id: u64,
        circuit_type: ProvingJobCircuitType,
        group_id: u32,
        sub_group_id: u32,
        task_index: u32,
    ) -> Self {
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
    pub fn new_groth16_proof_job_id(
        goal_id: u64,
        circuit_type: ProvingJobCircuitType,
        group_id: u32,
        sub_group_id: u32,
        task_index: u32,
    ) -> Self {
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
    pub fn notify_block_complete(block_id: u64) -> Self {
        Self {
            topic: QJobTopic::NotifyOrchestratorComplete,
            goal_id: block_id,
            group_id: 0,
            circuit_type: ProvingJobCircuitType::Unknown,
            sub_group_id: 0,
            task_index: 0,
            data_type: ProvingJobDataType::InputWitness,
            data_index: 0,
        }
    }
    pub fn block_agg_state_part_1_input_witness(block_id: u64) -> Self {
        Self {
            topic: QJobTopic::GenerateStandardProof,
            goal_id: block_id,
            group_id: ProvingJobCircuitType::AggUserRegisterDeployContractsGUTA
                .to_circuit_group_id(),
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
            group_id: ProvingJobCircuitType::AggAddProcessL1WithdrawalAddL1Deposit
                .to_circuit_group_id(),
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
            group_id: ProvingJobCircuitType::GenerateRollupStateTransitionProof
                .to_circuit_group_id(),
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
            group_id: ProvingJobCircuitType::GenerateSigHashIntrospectionProof
                .to_circuit_group_id(),
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
    pub fn is_notify_orchestrator_complete(&self) -> bool {
        self.topic == QJobTopic::NotifyOrchestratorComplete
    }

    pub fn is_notify_realm_complete(&self) -> bool {
        self.circuit_type == ProvingJobCircuitType::NotifyRealmComplete
    }

    pub fn is_notify_complete(&self) -> bool {
        self.is_notify_orchestrator_complete() || self.is_notify_realm_complete()
    }

    pub fn get_tree_parent_proof_input_id(&self) -> Self {
        let parent_type = match self.circuit_type {
            ProvingJobCircuitType::AppendUserRegistrationTree => {
                ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate => {
                ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::BatchDeployContracts => {
                ProvingJobCircuitType::BatchDeployContractsAggregate
            }
            ProvingJobCircuitType::BatchDeployContractsAggregate => {
                ProvingJobCircuitType::BatchDeployContractsAggregate
            }
            ProvingJobCircuitType::AddL1Deposit => ProvingJobCircuitType::AddL1DepositAggregate,
            ProvingJobCircuitType::AddL1DepositAggregate => {
                ProvingJobCircuitType::AddL1DepositAggregate
            }
            ProvingJobCircuitType::ClaimL1Deposit => ProvingJobCircuitType::ClaimL1DepositAggregate,
            ProvingJobCircuitType::ClaimL1DepositAggregate => {
                ProvingJobCircuitType::ClaimL1DepositAggregate
            }
            ProvingJobCircuitType::AddL1Withdrawal => {
                ProvingJobCircuitType::AddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::AddL1WithdrawalAggregate => {
                ProvingJobCircuitType::AddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::ProcessL1Withdrawal => {
                ProvingJobCircuitType::ProcessL1WithdrawalAggregate
            }
            ProvingJobCircuitType::ProcessL1WithdrawalAggregate => {
                ProvingJobCircuitType::ProcessL1WithdrawalAggregate
            }
            ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => {
                ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
            }
            ProvingJobCircuitType::DummyAddL1DepositAggregate => {
                ProvingJobCircuitType::AddL1DepositAggregate
            }
            ProvingJobCircuitType::DummyClaimL1DepositAggregate => {
                ProvingJobCircuitType::ClaimL1DepositAggregate
            }
            ProvingJobCircuitType::DummyAddL1WithdrawalAggregate => {
                ProvingJobCircuitType::AddL1WithdrawalAggregate
            }
            ProvingJobCircuitType::DummyProcessL1WithdrawalAggregate => {
                ProvingJobCircuitType::ProcessL1WithdrawalAggregate
            }
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
        Self {
            task_index,
            ..*self
        }
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
        Self {
            checkpoint_id,
            job_id,
        }
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
        Ok(Self {
            checkpoint_id,
            job_id,
        })
    }
}

pub trait QWorkerModeFilter {
    fn can_process_job(&self, job_id: QProvingJobDataID) -> bool;
}
impl QWorkerModeFilter for QWorkerMode {
    fn can_process_job(&self, job_id: QProvingJobDataID) -> bool {
        match *self {
            QWorkerMode::All => true,
            QWorkerMode::NoGroth16 => {
                job_id.circuit_type != ProvingJobCircuitType::WrapFinalSigHashProofBLS12381
            }
            QWorkerMode::OnlyGroth16 => {
                job_id.circuit_type == ProvingJobCircuitType::WrapFinalSigHashProofBLS12381
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let job =
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AddL1Deposit, 0, 0, 0);

        let result = bincode::serialize(&job).unwrap();

        let result2 = job.to_fixed_bytes();
        assert_eq!(result, result2.to_vec());

        let decoded_job: QProvingJobDataID = bincode::deserialize(&result).unwrap();

        assert_eq!(job, decoded_job);
        let decoded_job2: QProvingJobDataID = bincode::deserialize(&result2).unwrap();

        assert_eq!(job, decoded_job2);
    }

    #[test]
    fn test_jobs_task_graph_topological_sort() {
        let mut graph = JobsTaskGraph::new();

        let task1 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![QProvingJobDataID::new_proof_job_id(
                1,
                ProvingJobCircuitType::AddL1Deposit,
                0,
                0,
                0,
            )],
        };
        let task2 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![QProvingJobDataID::new_proof_job_id(
                1,
                ProvingJobCircuitType::AddL1Deposit,
                0,
                1,
                0,
            )],
        };
        let task3 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![QProvingJobDataID::new_proof_job_id(
                1,
                ProvingJobCircuitType::AddL1Deposit,
                0,
                2,
                0,
            )],
        };

        graph.add_dep(task3.clone(), task1.clone());
        graph.add_dep(task3.clone(), task2.clone());

        let sorted = graph.ts();

        let task1_pos = sorted.iter().position(|&t| t == task1.task_id).unwrap();
        let task2_pos = sorted.iter().position(|&t| t == task2.task_id).unwrap();
        let task3_pos = sorted.iter().position(|&t| t == task3.task_id).unwrap();

        assert!(task1_pos < task3_pos);
        assert!(task2_pos < task3_pos);
    }

    #[test]
    fn test_job_proof_verification() {
        use plonky2::field::goldilocks_field::GoldilocksField;
        use plonky2::hash::poseidon::PoseidonHash;
        use plonky2::plonk::config::Hasher;

        let graph = JobsTaskGraph::new();

        let job_id =
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AddL1Deposit, 0, 0, 0);
        let value = QHashOut::<GoldilocksField>::from_values(1, 2, 3, 4);

        let siblings = vec![
            JobProofSibling {
                hash: QHashOut::from_values(5, 6, 7, 8),
                is_left: true,
                parent_public_key: None,
            },
            JobProofSibling {
                hash: QHashOut::from_values(9, 10, 11, 12),
                is_left: false,
                parent_public_key: Some(QHashOut::from_values(13, 14, 15, 16)),
            },
        ];

        let mut current = value;
        for sibling in &siblings {
            current = if sibling.is_left {
                QHashOut(PoseidonHash::two_to_one(sibling.hash.0, current.0))
            } else {
                QHashOut(PoseidonHash::two_to_one(current.0, sibling.hash.0))
            };
        }

        let proof = JobProof {
            job_id,
            value,
            siblings,
            root: current,
        };

        assert!(graph.verify_proof(&proof));

        // Test with wrong root
        let wrong_proof = JobProof {
            job_id,
            value,
            siblings: proof.siblings.clone(),
            root: QHashOut::from_values(0, 0, 0, 0),
        };
        assert!(!graph.verify_proof(&wrong_proof));
    }

    #[test]
    #[should_panic(expected = "cycle detected")]
    fn test_cycle_detection() {
        let mut graph = JobsTaskGraph::new();

        let task1 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task2 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };

        graph.add_dep(task1.clone(), task2.clone());
        graph.add_dep(task2.clone(), task1.clone());

        graph.ts();
    }

    #[test]
    fn test_get_task_levels_simple() {
        let mut graph = JobsTaskGraph::new();

        let task1 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task2 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };

        graph.add_dep(task1.clone(), task2.clone());

        let levels = graph.get_task_levels();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 1);
        assert!(levels[0].contains(&task2.task_id));
        assert_eq!(levels[1].len(), 1);
        assert!(levels[1].contains(&task1.task_id));
    }

    #[test]
    fn test_get_task_levels_parallel() {
        let mut graph = JobsTaskGraph::new();

        let task1 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task2 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task3 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };

        graph.add_dep(task3.clone(), task1.clone());
        graph.add_dep(task3.clone(), task2.clone());

        let levels = graph.get_task_levels();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 2);
        assert!(levels[0].contains(&task1.task_id));
        assert!(levels[0].contains(&task2.task_id));
        assert_eq!(levels[1].len(), 1);
        assert!(levels[1].contains(&task3.task_id));
    }

    #[test]
    fn test_get_task_levels_diamond() {
        let mut graph = JobsTaskGraph::new();

        let task1 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task2 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task3 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task4 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };

        graph.add_dep(task2.clone(), task1.clone());
        graph.add_dep(task3.clone(), task1.clone());
        graph.add_dep(task4.clone(), task2.clone());
        graph.add_dep(task4.clone(), task3.clone());

        let levels = graph.get_task_levels();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].len(), 1);
        assert!(levels[0].contains(&task1.task_id));
        assert_eq!(levels[1].len(), 2);
        assert!(levels[1].contains(&task2.task_id));
        assert!(levels[1].contains(&task3.task_id));
        assert_eq!(levels[2].len(), 1);
        assert!(levels[2].contains(&task4.task_id));
    }

    #[test]
    fn test_get_task_levels_complex() {
        let mut graph = JobsTaskGraph::new();

        let task1 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task2 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task3 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task4 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task5 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };

        graph.add_dep(task2.clone(), task1.clone());
        graph.add_dep(task3.clone(), task2.clone());
        graph.add_dep(task4.clone(), task1.clone());
        graph.add_dep(task5.clone(), task3.clone());
        graph.add_dep(task5.clone(), task4.clone());

        let levels = graph.get_task_levels();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].len(), 1);
        assert!(levels[0].contains(&task1.task_id));
        assert_eq!(levels[1].len(), 2);
        assert!(levels[1].contains(&task2.task_id));
        assert!(levels[1].contains(&task4.task_id));
        assert_eq!(levels[2].len(), 2);
        assert!(levels[2].contains(&task3.task_id));
        assert!(levels[2].contains(&task5.task_id));
    }

    #[test]
    fn test_get_task_levels_empty() {
        let graph = JobsTaskGraph::new();
        let levels = graph.get_task_levels();
        assert_eq!(levels.len(), 0);
    }

    #[test]
    fn test_get_task_levels_odd_leaves() {
        let mut graph = JobsTaskGraph::new();

        let task1 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task2 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task3 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };
        let task4 = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };

        graph.add_dep(task4.clone(), task1.clone());
        graph.add_dep(task4.clone(), task2.clone());
        graph.add_dep(task4.clone(), task3.clone());

        let levels = graph.get_task_levels();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 3);
        assert!(levels[0].contains(&task1.task_id));
        assert!(levels[0].contains(&task2.task_id));
        assert!(levels[0].contains(&task3.task_id));
        assert_eq!(levels[1].len(), 1);
        assert!(levels[1].contains(&task4.task_id));
    }

    #[test]
    fn test_get_task_levels_single_task() {
        let mut graph = JobsTaskGraph::new();

        let task = JobsTask {
            task_id: TaskId::new(),
            job_ids: vec![],
        };

        graph.tasks.insert(task.task_id, task.clone());

        let levels = graph.get_task_levels();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 1);
        assert_eq!(levels[0][0], task.task_id);
    }
}
