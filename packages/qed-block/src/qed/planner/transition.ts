import { getDummyTreeProverIdsOpCircuit } from "./dummyTree";
import { ICityOpJobConfig } from "../bench/types";
import { IQProvingJobDataID, ProvingJobCircuitType } from "../job/id";

interface ICityOpJobIds {
  register_user_job_ids: IQProvingJobDataID[][];
  claim_deposit_job_ids: IQProvingJobDataID[][];
  token_transfer_job_ids: IQProvingJobDataID[][];
  add_withdrawal_job_ids: IQProvingJobDataID[][];

  process_withdrawal_job_ids: IQProvingJobDataID[][];
  add_deposit_job_ids: IQProvingJobDataID[][];
}




function getDummyOpJobIds(checkpointId: number, config: ICityOpJobConfig): ICityOpJobIds {

  let register_user_job_ids = getDummyTreeProverIdsOpCircuit(
    ProvingJobCircuitType.RegisterUser,
    ProvingJobCircuitType.DummyRegisterUserAggregate,
    checkpointId,
    config.register_user_count,
  );
  let claim_deposit_job_ids = getDummyTreeProverIdsOpCircuit(
    ProvingJobCircuitType.ClaimL1Deposit,
    ProvingJobCircuitType.DummyClaimL1DepositAggregate,
    checkpointId,
    config.claim_deposit_count,
  );
  let token_transfer_job_ids = getDummyTreeProverIdsOpCircuit(
    ProvingJobCircuitType.TransferTokensL2,
    ProvingJobCircuitType.DummyTransferTokensL2Aggregate,
    checkpointId,
    config.token_transfer_count,
  );
  let add_withdrawal_job_ids = getDummyTreeProverIdsOpCircuit(
    ProvingJobCircuitType.AddL1Withdrawal,
    ProvingJobCircuitType.DummyAddL1WithdrawalAggregate,
    checkpointId,
    config.add_withdrawal_count,
  );
  let process_withdrawal_job_ids = getDummyTreeProverIdsOpCircuit(
    ProvingJobCircuitType.ProcessL1Withdrawal,
    ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate,
    checkpointId,
    config.process_withdrawal_count,
  );
  let add_deposit_job_ids = getDummyTreeProverIdsOpCircuit(
    ProvingJobCircuitType.AddL1Deposit,
    ProvingJobCircuitType.DummyAddL1DepositAggregate,
    checkpointId,
    config.add_deposit_count,
  );
  return {
    register_user_job_ids,
    claim_deposit_job_ids,
    token_transfer_job_ids,
    add_withdrawal_job_ids,
    process_withdrawal_job_ids,
    add_deposit_job_ids,
  }
}

export type {
  ICityOpJobIds,
}
export {
  getDummyOpJobIds,
}