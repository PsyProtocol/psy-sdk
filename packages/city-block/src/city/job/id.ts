import { hexToU8Array, u8ArrayToHex } from "../../utils/data";

enum QJobTopic {
  GenerateStandardProof = 0,
  GenerateGroth16Proof = 1,
  BlockUserSignatureProof = 2,
  NotifyOrchestratorComplete = 3,
  AggregateJobs = 4,
}

enum ProvingJobDataType {
  InputWitness = 0,
  BaseInputProof = 1,
  OutputProof = 8,
  Counter = 16,
}

enum ProvingJobCircuitType {
  RegisterUser = 0,
  RegisterUserAggregate = 1,

  AddL1Deposit = 2,
  AddL1DepositAggregate = 3,

  ClaimL1Deposit = 4,
  ClaimL1DepositAggregate = 5,

  TransferTokensL2 = 6,
  TransferTokensL2Aggregate = 7,

  AddL1Withdrawal = 8,
  AddL1WithdrawalAggregate = 9,

  ProcessL1Withdrawal = 10,
  ProcessL1WithdrawalAggregate = 11,

  GenerateRollupStateTransitionProof = 32,
  GenerateSigHashIntrospectionProof = 33,
  GenerateFinalSigHashProof = 34,
  GenerateFinalSigHashProofGroth16 = 35,
  WrapFinalSigHashProofBLS12381 = 36,

  AggUserRegisterClaimDepositL2Transfer = 40,
  AggAddProcessL1WithdrawalAddL1Deposit = 41,

  DummyRegisterUserAggregate = 48,
  DummyAddL1DepositAggregate = 49,
  DummyClaimL1DepositAggregate = 50,
  DummyTransferTokensL2Aggregate = 51,
  DummyAddL1WithdrawalAggregate = 52,
  DummyProcessL1WithdrawalAggregate = 53,

  WrappedSignatureProof = 64,
  Secp256K1SignatureProof = 65,
  Unknown = 255,
}

interface IQProvingJobDataID {
  topic: QJobTopic,
  goal_id: number, // goal_id is u64, but block number should not exceed 2^53-1 (Number.MAX_SAFE_INTEGER)
  circuit_type: ProvingJobCircuitType,
  group_id: number,
  sub_group_id: number,
  task_index: number,
  data_type: ProvingJobDataType,
  data_index: number,
}
/*


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


*/
function deserializeJobId(jobId: Uint8Array | number [] | string): IQProvingJobDataID {
  if(typeof jobId === 'string'){
    return deserializeJobId(hexToU8Array(jobId));
  }else if(!(jobId as Uint8Array).buffer){
    return deserializeJobId(new Uint8Array(jobId as number[]));
  }

  const dataView = new DataView((jobId as Uint8Array).buffer);
  const topic = dataView.getUint8(0);
  const goal_id = Number(dataView.getBigUint64(1).toString());
  const circuit_type = dataView.getUint8(9);
  const group_id = dataView.getUint32(10, true);
  const sub_group_id = dataView.getUint32(14, true);
  const task_index = dataView.getUint32(18, true);
  const data_type = dataView.getUint8(22);
  const data_index = dataView.getUint8(23);
  return {
    topic,
    goal_id,
    circuit_type,
    group_id,
    sub_group_id,
    task_index,
    data_type,
    data_index,
  };
}

function serializeJobId(jobId: IQProvingJobDataID){
  const buffer = new ArrayBuffer(24);
  const dataView = new DataView(buffer);
  dataView.setUint8(0, jobId.topic);
  dataView.setBigUint64(1, BigInt(jobId.goal_id));
  dataView.setUint8(9, jobId.circuit_type);
  dataView.setUint32(10, jobId.group_id, true);
  dataView.setUint32(14, jobId.sub_group_id, true);
  dataView.setUint32(18, jobId.task_index, true);
  dataView.setUint8(22, jobId.data_type);
  dataView.setUint8(23, jobId.data_index);
  return new Uint8Array(buffer);
}

function serializeJobIdHex(jobId: IQProvingJobDataID){
  return u8ArrayToHex(serializeJobId(jobId));
}

export {
  deserializeJobId,
  serializeJobId,
  serializeJobIdHex,
  ProvingJobDataType,
  ProvingJobCircuitType,
  QJobTopic,
}

export type {
  IQProvingJobDataID,
}