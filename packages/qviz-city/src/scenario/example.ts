import {
  ProvingJobCircuitType,
  createBinaryTreePlanner,
  depSerializedToProofNodes,
  getDummyTreeOpCircuitJobWithDependencies,
  getDummyTreeProverIdsOpCircuit,
  synthPlanner,
} from "@qstudio/city-block";
import { ISimpleCityBlock } from "@qstudio/city-block";

const transferTree = getDummyTreeOpCircuitJobWithDependencies(
  ProvingJobCircuitType.TransferTokensL2,
  ProvingJobCircuitType.DummyTransferTokensL2Aggregate,
  0,
  4,
);

const EXAMPLE_SCENARIO: ISimpleCityBlock = {
  stateTransitionRoot: transferTree,
  sighashProofs: [],
};
const simpleBlock = synthPlanner({
  checkpoint_id: 2,
  job_config: {
    register_user_count: 2,
    claim_deposit_count: 2,
    token_transfer_count: 2,
    add_withdrawal_count: 2,
    process_withdrawal_count: 2,
    add_deposit_count: 2,
  },
});
const EXAMPLE_SCENARIO_2: ISimpleCityBlock = {
  stateTransitionRoot: depSerializedToProofNodes(simpleBlock.root_state_transition),
  sighashProofs: simpleBlock.sighash_proofs,
};

export { EXAMPLE_SCENARIO, EXAMPLE_SCENARIO_2 };
