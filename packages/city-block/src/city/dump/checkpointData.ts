import { CityRPCProvider } from "@qstudio/city-sdk";

import {
  getJobSubGroupCounterGoalId,
  newCoreOpWitnessJobId,
  newProofJobId,
  ProvingJobCircuitType,
  serializeJobIdHex,
} from "../job";
import { IDumpProofStoreConfig } from "../bench/types";
import { hexToU8Array } from "../../utils/data";

async function getLeafCountOrDummy(
  getPSBytes: (key: string) => Promise<Uint8Array>,
  circuitType: ProvingJobCircuitType,
  dummyType: ProvingJobCircuitType,
  checkpointId: number
): Promise<number> {
  const counterJobId = getJobSubGroupCounterGoalId(
    newCoreOpWitnessJobId(circuitType, checkpointId, 0)
  );
  const dummyJobId = newProofJobId(checkpointId, dummyType, 0xdd, 0, 0);
  try {
    const counterBytes = await getPSBytes(serializeJobIdHex(counterJobId));
    if (counterBytes.length === 4) {
      return new Uint32Array(new Uint8Array(counterBytes).buffer)[0];
    }
  } catch (err) {}

  const dummyBytes = await getPSBytes(serializeJobIdHex(dummyJobId));
  if (dummyBytes.length !== 0) {
    return 0;
  } else {
    throw new Error(
      `no counter or dummy job found for circuit type ${circuitType} and checkpoint_id ${checkpointId}`
    );
  }
}

async function getProofStoreConfigPS(
  getPSBytes: (key: string) => Promise<Uint8Array>,
  checkpointId: number,
  rpcNodeId: number
): Promise<IDumpProofStoreConfig> {
  const register_user_count = await getLeafCountOrDummy(
    getPSBytes,
    ProvingJobCircuitType.RegisterUser,
    ProvingJobCircuitType.DummyRegisterUserAggregate,
    checkpointId
  );
  const add_deposit_count = await getLeafCountOrDummy(
    getPSBytes,
    ProvingJobCircuitType.AddL1Deposit,
    ProvingJobCircuitType.DummyAddL1DepositAggregate,
    checkpointId
  );
  const token_transfer_count = await getLeafCountOrDummy(
    getPSBytes,
    ProvingJobCircuitType.TransferTokensL2,
    ProvingJobCircuitType.DummyTransferTokensL2Aggregate,
    checkpointId
  );
  const add_withdrawal_count = await getLeafCountOrDummy(
    getPSBytes,
    ProvingJobCircuitType.AddL1Withdrawal,
    ProvingJobCircuitType.DummyAddL1WithdrawalAggregate,
    checkpointId
  );
  const process_withdrawal_count = await getLeafCountOrDummy(
    getPSBytes,
    ProvingJobCircuitType.ProcessL1Withdrawal,
    ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate,
    checkpointId
  );
  const claim_deposit_count = await getLeafCountOrDummy(
    getPSBytes,
    ProvingJobCircuitType.ClaimL1Deposit,
    ProvingJobCircuitType.DummyClaimL1DepositAggregate,
    checkpointId
  );
  return {
    checkpoint_id: checkpointId,
    rpc_node_id: rpcNodeId,
    job_config: {
      register_user_count,
      claim_deposit_count,
      token_transfer_count,
      add_withdrawal_count,
      process_withdrawal_count,
      add_deposit_count,
    },
  };
}

async function getProofStoreConfigForCheckpoint(
  rpc: CityRPCProvider,
  checkpointId: number,
  rpcNodeId: number
): Promise<IDumpProofStoreConfig> {
  const getPSBytes = async (key: string) => {
    const result = await rpc.getProofStoreValue(key);
    return result.length ? hexToU8Array(result) : new Uint8Array(0);
  };

  return getProofStoreConfigPS(getPSBytes, checkpointId, rpcNodeId);
}

export {
  getProofStoreConfigForCheckpoint,
  getLeafCountOrDummy,
  getProofStoreConfigPS,
};
