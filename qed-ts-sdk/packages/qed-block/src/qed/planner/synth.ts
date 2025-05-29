import { getDummyTreeOpCircuitJobWithDependencies } from "./dummyTree";
import { ICitySynthBlockConfig } from "../bench";
import { ICitySighashGroth16ProofResult, ICitySynthBlockResult, IQJobWithDependenciesSerialized } from "../bench/types";
import {
    ProvingJobCircuitType,
    blockAggStatePart1InputWitnessJobId,
    blockAggStatePart2InputWitnessJobId,
    blockStateTransitionInputWitnessJobId,
    serializeJobIdHex,
    sighashFinalInputWitnessJobId,
    sighashIntrospectionInputWitnessJobId,
    wrapSighashFinalBls3812InputWitnessJobId,
} from "../job";

function synthPlanner(config: ICitySynthBlockConfig): ICitySynthBlockResult {
    const checkpointId = config.checkpoint_id;

    const registerUsers = getDummyTreeOpCircuitJobWithDependencies(
        ProvingJobCircuitType.RegisterUser,
        ProvingJobCircuitType.DummyRegisterUserAggregate,
        checkpointId,
        config.job_config.register_user_count
    );
    const claimDeposits = getDummyTreeOpCircuitJobWithDependencies(
        ProvingJobCircuitType.ClaimL1Deposit,
        ProvingJobCircuitType.DummyClaimL1DepositAggregate,
        checkpointId,
        config.job_config.claim_deposit_count
    );
    const tokenTransfers = getDummyTreeOpCircuitJobWithDependencies(
        ProvingJobCircuitType.TransferTokensL2,
        ProvingJobCircuitType.DummyTransferTokensL2Aggregate,
        checkpointId,
        config.job_config.token_transfer_count
    );
    const addWithdrawals = getDummyTreeOpCircuitJobWithDependencies(
        ProvingJobCircuitType.AddL1Withdrawal,
        ProvingJobCircuitType.DummyAddL1WithdrawalAggregate,
        checkpointId,
        config.job_config.add_withdrawal_count
    );
    const processWithdrawals = getDummyTreeOpCircuitJobWithDependencies(
        ProvingJobCircuitType.ProcessL1Withdrawal,
        ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate,
        checkpointId,
        config.job_config.process_withdrawal_count
    );
    const addDeposits = getDummyTreeOpCircuitJobWithDependencies(
        ProvingJobCircuitType.AddL1Deposit,
        ProvingJobCircuitType.DummyAddL1DepositAggregate,
        checkpointId,
        config.job_config.add_deposit_count
    );

    const stateTransitionPart1Id = serializeJobIdHex(blockAggStatePart1InputWitnessJobId(checkpointId));
    const stateTransitionPart2Id = serializeJobIdHex(blockAggStatePart2InputWitnessJobId(checkpointId));

    const stateTransitionPart1: IQJobWithDependenciesSerialized = {
        id: stateTransitionPart1Id,
        dependencies: [registerUsers, claimDeposits, tokenTransfers],
    };
    const stateTransitionPart2: IQJobWithDependenciesSerialized = {
        id: stateTransitionPart2Id,
        dependencies: [addWithdrawals, processWithdrawals, addDeposits],
    };

    const rootStateTransitionId = serializeJobIdHex(blockStateTransitionInputWitnessJobId(checkpointId));
    const root_state_transition: IQJobWithDependenciesSerialized = {
        id: rootStateTransitionId,
        dependencies: [stateTransitionPart1, stateTransitionPart2],
    };

    const introspectionJobsCount = config.job_config.add_deposit_count + 1;
    const sighash_proofs: ICitySighashGroth16ProofResult[] = [];
    for (let i = 0; i < introspectionJobsCount; i++) {
        sighash_proofs[i] = {
            sighash_introspection: serializeJobIdHex(sighashIntrospectionInputWitnessJobId(checkpointId, i)),
            sighash_final: serializeJobIdHex(sighashFinalInputWitnessJobId(checkpointId, i)),
            groth16_final: serializeJobIdHex(wrapSighashFinalBls3812InputWitnessJobId(checkpointId, i)),
            state_transition_reference: rootStateTransitionId,
        };
    }

    return {
        root_state_transition,
        sighash_proofs,
    };
}

export { synthPlanner };
