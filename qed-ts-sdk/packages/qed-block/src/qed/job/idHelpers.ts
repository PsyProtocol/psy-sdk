import { CityJobNames, CityWidgetNames } from "./humanReadable";
import {
    IQProvingJobDataID,
    ProvingJobCircuitType,
    ProvingJobDataType,
    QJobTopic,
    deserializeJobId,
    getGroupIdForCircuitType,
    serializeJobIdHex,
} from "./id";

function newCoreOpWitnessJobId(
    circuit_type: ProvingJobCircuitType,
    checkpoint_id: number,
    task_index: number
): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id: checkpoint_id,
        circuit_type,
        group_id: getGroupIdForCircuitType(circuit_type),
        sub_group_id: 0,
        task_index,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function newTransferSignatureProofJobId(
    rpc_node_id: number,
    block_id: number,
    transfer_id: number
): IQProvingJobDataID {
    return {
        topic: QJobTopic.BlockUserSignatureProof,
        goal_id: block_id,
        group_id: 1,
        circuit_type: ProvingJobCircuitType.WrappedSignatureProof,
        sub_group_id: rpc_node_id,
        task_index: transfer_id,
        data_type: ProvingJobDataType.BaseInputProof,
        data_index: 0,
    };
}
function newWithdrawalSignatureProofJobId(
    rpc_node_id: number,
    block_id: number,
    withdrawal_id: number
): IQProvingJobDataID {
    return {
        topic: QJobTopic.BlockUserSignatureProof,
        goal_id: block_id,
        group_id: 2,
        circuit_type: ProvingJobCircuitType.WrappedSignatureProof,
        sub_group_id: rpc_node_id,
        task_index: withdrawal_id,
        data_type: ProvingJobDataType.BaseInputProof,
        data_index: 0,
    };
}
function newClaimDepositL1SignatureProofJobId(
    rpc_node_id: number,
    block_id: number,
    deposit_id: number
): IQProvingJobDataID {
    return {
        topic: QJobTopic.BlockUserSignatureProof,
        goal_id: block_id,
        group_id: 3,
        circuit_type: ProvingJobCircuitType.Secp256K1SignatureProof,
        sub_group_id: rpc_node_id,
        task_index: deposit_id,
        data_type: ProvingJobDataType.BaseInputProof,
        data_index: 0,
    };
}
function newProofJobId(
    goal_id: number,
    circuit_type: ProvingJobCircuitType,
    group_id: number,
    sub_group_id: number,
    task_index: number
): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id,
        circuit_type,
        group_id,
        sub_group_id,
        task_index,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function newGroth16ProofJobId(
    goal_id: number,
    circuit_type: ProvingJobCircuitType,
    group_id: number,
    sub_group_id: number,
    task_index: number
): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateGroth16Proof,
        goal_id,
        circuit_type,
        group_id,
        sub_group_id,
        task_index,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function getBlockAggregateJobsGroupJobId(block_id: number, group_id: number, task_index: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.AggregateJobs,
        goal_id: block_id,
        group_id,
        circuit_type: ProvingJobCircuitType.Unknown,
        sub_group_id: 0,
        task_index,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function notifyBlockCompleteJobId(block_id: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.NotifyOrchestratorComplete,
        goal_id: block_id,
        group_id: 0,
        circuit_type: ProvingJobCircuitType.Unknown,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function blockAggStatePart1InputWitnessJobId(block_id: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id: block_id,
        group_id: getGroupIdForCircuitType(ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer),
        circuit_type: ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function blockAggStatePart2InputWitnessJobId(block_id: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id: block_id,
        group_id: getGroupIdForCircuitType(ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit),
        circuit_type: ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function blockStateTransitionInputWitnessJobId(block_id: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id: block_id,
        group_id: getGroupIdForCircuitType(ProvingJobCircuitType.GenerateRollupStateTransitionProof),
        circuit_type: ProvingJobCircuitType.GenerateRollupStateTransitionProof,
        sub_group_id: 0,
        task_index: 0,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function sighashIntrospectionInputWitnessJobId(block_id: number, input_id: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id: block_id,
        group_id: getGroupIdForCircuitType(ProvingJobCircuitType.GenerateSigHashIntrospectionProof),
        circuit_type: ProvingJobCircuitType.GenerateSigHashIntrospectionProof,
        sub_group_id: 0,
        task_index: input_id,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function sighashFinalInputWitnessJobId(block_id: number, input_id: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id: block_id,
        group_id: getGroupIdForCircuitType(ProvingJobCircuitType.GenerateFinalSigHashProof),
        circuit_type: ProvingJobCircuitType.GenerateFinalSigHashProof,
        sub_group_id: input_id,
        task_index: input_id,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function wrapSighashFinalBls3812InputWitnessJobId(block_id: number, input_id: number): IQProvingJobDataID {
    return {
        topic: QJobTopic.GenerateStandardProof,
        goal_id: block_id,
        group_id: getGroupIdForCircuitType(ProvingJobCircuitType.WrapFinalSigHashProofBLS12381),
        circuit_type: ProvingJobCircuitType.WrapFinalSigHashProofBLS12381,
        sub_group_id: input_id,
        task_index: input_id,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    };
}
function getJobInputProofId(jobId: IQProvingJobDataID, data_index: number): IQProvingJobDataID {
    return {
        ...jobId,
        data_type: ProvingJobDataType.BaseInputProof,
        data_index,
    };
}
function isNotifyOrchestratorCompleteJobId(jobId: IQProvingJobDataID): boolean {
    return jobId.topic === QJobTopic.NotifyOrchestratorComplete;
}

function getJobOutputId(jobId: IQProvingJobDataID): IQProvingJobDataID {
    return {
        ...jobId,
        data_type: ProvingJobDataType.OutputProof,
        data_index: 0,
    };
}
function getJobSubGroupCounterId(jobId: IQProvingJobDataID): IQProvingJobDataID {
    return {
        ...jobId,
        data_type: ProvingJobDataType.Counter,
        task_index: 0,
        data_index: 0,
    };
}
function getJobSubGroupCounterGoalId(jobId: IQProvingJobDataID): IQProvingJobDataID {
    return {
        ...jobId,
        data_type: ProvingJobDataType.Counter,
        task_index: 0,
        data_index: 1,
    };
}
function getSubGroupCounterGoalNextJobsId(jobId: IQProvingJobDataID): IQProvingJobDataID {
    return {
        ...jobId,
        data_type: ProvingJobDataType.Counter,
        task_index: 0,
        data_index: 2,
    };
}
function jobIdWithTaskIndex(jobId: IQProvingJobDataID, task_index: number): IQProvingJobDataID {
    return {
        ...jobId,
        task_index,
    };
}

function getJobTreeParentProofInputId(id: IQProvingJobDataID): IQProvingJobDataID {
    let parent_type: ProvingJobCircuitType = id.circuit_type;
    switch (id.circuit_type) {
        case ProvingJobCircuitType.RegisterUser:
            parent_type = ProvingJobCircuitType.RegisterUserAggregate;
            break;
        case ProvingJobCircuitType.RegisterUserAggregate:
            parent_type = ProvingJobCircuitType.RegisterUserAggregate;
            break;
        case ProvingJobCircuitType.AddL1Deposit:
            parent_type = ProvingJobCircuitType.AddL1DepositAggregate;
            break;
        case ProvingJobCircuitType.AddL1DepositAggregate:
            parent_type = ProvingJobCircuitType.AddL1DepositAggregate;
            break;
        case ProvingJobCircuitType.ClaimL1Deposit:
            parent_type = ProvingJobCircuitType.ClaimL1DepositAggregate;
            break;
        case ProvingJobCircuitType.ClaimL1DepositAggregate:
            parent_type = ProvingJobCircuitType.ClaimL1DepositAggregate;
            break;
        case ProvingJobCircuitType.TransferTokensL2:
            parent_type = ProvingJobCircuitType.TransferTokensL2Aggregate;
            break;
        case ProvingJobCircuitType.TransferTokensL2Aggregate:
            parent_type = ProvingJobCircuitType.TransferTokensL2Aggregate;
            break;
        case ProvingJobCircuitType.AddL1Withdrawal:
            parent_type = ProvingJobCircuitType.AddL1WithdrawalAggregate;
            break;
        case ProvingJobCircuitType.AddL1WithdrawalAggregate:
            parent_type = ProvingJobCircuitType.AddL1WithdrawalAggregate;
            break;
        case ProvingJobCircuitType.ProcessL1Withdrawal:
            parent_type = ProvingJobCircuitType.ProcessL1WithdrawalAggregate;
            break;
        case ProvingJobCircuitType.ProcessL1WithdrawalAggregate:
            parent_type = ProvingJobCircuitType.ProcessL1WithdrawalAggregate;
            break;
        case ProvingJobCircuitType.DummyRegisterUserAggregate:
            parent_type = ProvingJobCircuitType.RegisterUserAggregate;
            break;
        case ProvingJobCircuitType.DummyAddL1DepositAggregate:
            parent_type = ProvingJobCircuitType.AddL1DepositAggregate;
            break;
        case ProvingJobCircuitType.DummyClaimL1DepositAggregate:
            parent_type = ProvingJobCircuitType.ClaimL1DepositAggregate;
            break;
        case ProvingJobCircuitType.DummyTransferTokensL2Aggregate:
            parent_type = ProvingJobCircuitType.TransferTokensL2Aggregate;
            break;
        case ProvingJobCircuitType.DummyAddL1WithdrawalAggregate:
            parent_type = ProvingJobCircuitType.AddL1WithdrawalAggregate;
            break;
        case ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate:
            parent_type = ProvingJobCircuitType.ProcessL1WithdrawalAggregate;
            break;
    }
    return {
        ...id,
        //group_id: getGroupIdForCircuitType(parent_type),
        circuit_type: parent_type,
        sub_group_id: id.sub_group_id + 1,
        task_index: id.task_index >> 1,
    };
}

function getCircuitNameForJobId(jobId: IQProvingJobDataID): string {
    return CityJobNames[jobId.circuit_type];
}
function getCircuitNameForJobIdHex(jobId: string): string {
    return CityJobNames[deserializeJobId(jobId).circuit_type];
}
function getCircuitWidgetNameForJobId(jobId: IQProvingJobDataID): string {
    return CityWidgetNames[jobId.circuit_type];
}
function getCircuitWidgetNameForJobIdHex(jobId: string): string {
    return CityWidgetNames[deserializeJobId(jobId).circuit_type];
}
function getJobWitnessIdHex(jobIdHex: string): string {
    const jobId = deserializeJobId(jobIdHex);
    return serializeJobIdHex({
        ...jobId,
        data_type: ProvingJobDataType.InputWitness,
        data_index: 0,
    });
}
export {
    newCoreOpWitnessJobId,
    newTransferSignatureProofJobId,
    newWithdrawalSignatureProofJobId,
    newClaimDepositL1SignatureProofJobId,
    newProofJobId,
    newGroth16ProofJobId,
    getBlockAggregateJobsGroupJobId,
    notifyBlockCompleteJobId,
    blockAggStatePart1InputWitnessJobId,
    blockAggStatePart2InputWitnessJobId,
    blockStateTransitionInputWitnessJobId,
    sighashIntrospectionInputWitnessJobId,
    sighashFinalInputWitnessJobId,
    wrapSighashFinalBls3812InputWitnessJobId,
    getJobInputProofId,
    isNotifyOrchestratorCompleteJobId,
    getJobOutputId,
    getJobSubGroupCounterId,
    getJobSubGroupCounterGoalId,
    getSubGroupCounterGoalNextJobsId,
    jobIdWithTaskIndex,
    getJobTreeParentProofInputId,
    getCircuitNameForJobId,
    getCircuitNameForJobIdHex,
    getCircuitWidgetNameForJobId,
    getCircuitWidgetNameForJobIdHex,
    getJobWitnessIdHex,
};
