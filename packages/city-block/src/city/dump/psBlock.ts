import { CityRPCProviderWithCache, ICityRPCProvider } from "@qstudio/city-sdk";
import { depSerializedToProofNodes } from "../bench/dependencyResolver";
import { ICityOpJobConfig, ICitySynthBlockResult, IDumpProofStoreConfig, ISimpleCityBlock } from "../bench/types";
import { synthPlanner } from "../planner/synth";
import {IDogeLinkElectrsRPC} from "doge-sdk";
class PSCityBlock {
  checkpoint_id: number;
  rpc_node_id: number;
  job_config: ICityOpJobConfig;
  rpc: ICityRPCProvider;
  dogeRPC: IDogeLinkElectrsRPC;
  synthResult: ICitySynthBlockResult;
  blockExplorerUrl: string;
  jobIdWitnessCache: Record<string, any> = {};


  constructor(rpc: ICityRPCProvider, dogeRPC: IDogeLinkElectrsRPC, synthResult: ICitySynthBlockResult, dumpConfig: IDumpProofStoreConfig, blockExplorerUrl = "http://localhost:1337/explorer") {
    this.checkpoint_id = dumpConfig.checkpoint_id;
    this.rpc_node_id = dumpConfig.rpc_node_id;
    this.job_config = dumpConfig.job_config;
    this.rpc = new CityRPCProviderWithCache(rpc);
    this.dogeRPC = dogeRPC;
    this.synthResult = synthResult;
    this.blockExplorerUrl = blockExplorerUrl;
  }

  getSimpleCityBlock(): ISimpleCityBlock {
    const synth =  synthPlanner({
      checkpoint_id: this.checkpoint_id,
      job_config: {...this.job_config},
    });
    return {
      stateTransitionRoot: depSerializedToProofNodes(synth.root_state_transition),
      sighashProofs: synth.sighash_proofs,
    }
  }
  async loadJobWitness(jobId: string): Promise<any> {
    if(Object.hasOwnProperty.call(this.jobIdWitnessCache, jobId)) {
      return this.jobIdWitnessCache[jobId];
    }
    const value = await this.rpc.getProofStoreJobWitness(jobId);
    this.jobIdWitnessCache[jobId] = value;
    return value;
  }
}

export {
  PSCityBlock,
}