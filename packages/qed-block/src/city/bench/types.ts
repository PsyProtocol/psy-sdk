import { IQProvingJobDataID, ProvingJobCircuitType, deserializeJobId, serializeJobIdHex } from "../job/id";

interface ICityAggregatedOpJobCircuitType {
  leaf: ProvingJobCircuitType,
  agg: ProvingJobCircuitType,
  dummy: ProvingJobCircuitType,
  has_events: boolean,
}

interface ICityOpJobConfig {
  register_user_count: number;
  claim_deposit_count: number;
  token_transfer_count: number;
  add_withdrawal_count: number;
  process_withdrawal_count: number;
  add_deposit_count: number;
}
interface IDumpProofStoreConfig {
  checkpoint_id: number;
  rpc_node_id: number;
  job_config: ICityOpJobConfig;
}
interface IQJobWitnessWithId {
  job_id: IQProvingJobDataID;
  witness: any;
}
interface IQJobProofPublicInputs {
  job_id: IQProvingJobDataID;
  public_inputs: (string|number)[];
}
interface IQInspectDumpOutput {
  dependency_map?: IQJobWithDependenciesSerialized,
  job_config?: IDumpProofStoreConfig;
  signature_proof_dependency_ids?: IQProvingJobDataID[];
  proof_witnesses?: IQJobWitnessWithId[];
  proof_public_inputs?: IQJobProofPublicInputs[];
}
interface IQJobWithDependencies {
  id: IQProvingJobDataID;
  dependencies: IQJobWithDependencies[];
}

interface IQJobWithDependenciesSerialized {
  id: string;
  dependencies: IQJobWithDependenciesSerialized[];
}

interface ICitySynthBlockConfig {
  checkpoint_id: number;
  job_config: ICityOpJobConfig;
}

interface ICitySighashGroth16ProofResult {
  sighash_introspection: string;
  sighash_final: string;
  groth16_final: string;
  state_transition_reference: string;
}

interface ICSProofNode {
  id: string;
  dependencies: ICSProofNode[];
  is_ref?: boolean;
}
interface ISimpleCityBlock {
  stateTransitionRoot: ICSProofNode;
  sighashProofs: ICitySighashGroth16ProofResult[];
}

interface ICitySighashGroth16ProofResult {
  sighash_introspection: string;
  sighash_final: string;
  groth16_final: string;
  state_transition_reference: string;
}

interface ICitySynthBlockResult {
  root_state_transition: IQJobWithDependenciesSerialized;
  sighash_proofs: ICitySighashGroth16ProofResult[];
}

function deserializeJobWithDependencies(job: IQJobWithDependenciesSerialized): IQJobWithDependencies {
  return {
    id: deserializeJobId(job.id),
    dependencies: job.dependencies.map(x=>deserializeJobWithDependencies(x))
  }
}
function serializeJobWithDependencies(job: IQJobWithDependencies): IQJobWithDependenciesSerialized {
  return {
    id: serializeJobIdHex(job.id),
    dependencies: job.dependencies.map(x=>serializeJobWithDependencies(x))
  }
}

export {
  serializeJobWithDependencies,
  deserializeJobWithDependencies,
};

export type {
  IQJobWithDependencies,
  IQJobWithDependenciesSerialized,
  IQInspectDumpOutput,
  IQJobProofPublicInputs,
  IQJobWitnessWithId,
  IDumpProofStoreConfig,
  ICitySynthBlockConfig,
  ICityOpJobConfig,
  ICityAggregatedOpJobCircuitType,
  ICitySynthBlockResult,
  ICitySighashGroth16ProofResult,
  ICSProofNode,
  ISimpleCityBlock,
};