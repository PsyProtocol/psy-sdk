import { CityRPCProviderWithCache, ICityRPCProvider } from "@qed/qed-ts-sdk";
import { depSerializedToProofNodes } from "../bench/dependencyResolver";
import {
    ICityOpJobConfig,
    ICitySynthBlockResult,
    IDumpProofStoreConfig,
    IQJobWithDependenciesSerialized,
    ISimpleCityBlock,
} from "../bench/types";
import { synthPlanner } from "../planner/synth";
import { IDogeLinkElectrsRPC } from "doge-sdk";
import { ICityJobWitness } from "./witnessTypes";
import { deserializeJobId, getJobWitnessIdHex, ProvingJobCircuitType } from "../job";
function normailzeRST(rst: IQJobWithDependenciesSerialized): IQJobWithDependenciesSerialized {
    return {
        id: getJobWitnessIdHex(rst.id),
        dependencies: rst.dependencies.map((x) => normailzeRST(x)),
    };
}
function normalizeSynth(synth: ICitySynthBlockResult): ICitySynthBlockResult {
    return {
        root_state_transition: normailzeRST(synth.root_state_transition),
        sighash_proofs: synth.sighash_proofs.map((x) => ({
            sighash_final: getJobWitnessIdHex(x.sighash_final),
            groth16_final: getJobWitnessIdHex(x.groth16_final),
            sighash_introspection: getJobWitnessIdHex(x.sighash_introspection),
            state_transition_reference: getJobWitnessIdHex(x.state_transition_reference),
        })),
    };
}

function findInRST(jobId: string, rst: IQJobWithDependenciesSerialized): IQJobWithDependenciesSerialized | null {
    if (rst.id === jobId) {
        return rst;
    } else {
        for (let dep of rst.dependencies) {
            const found = findInRST(jobId, dep);
            if (found) {
                return found;
            }
        }
        return null;
        //console.log(deserializeJobId(jobId));
        //throw new Error("job id "+jobId+" not found in rst");
    }
}

function getFlatDependenciesForRST(rst: IQJobWithDependenciesSerialized): string[] {
    const direct = rst.dependencies.map((x) => x.id);
    const indirect = rst.dependencies.flatMap((x) => getFlatDependenciesForRST(x));
    return Array.from(new Set(direct.concat(indirect)));
}
class PSCityBlock<T> {
    checkpoint_id: number;
    rpc_node_id: number;
    job_config: ICityOpJobConfig;
    rpc: ICityRPCProvider;
    dogeRPC: IDogeLinkElectrsRPC;
    synthResult: ICitySynthBlockResult;
    blockExplorerUrl: string;
    jobIdWitnessCache: Record<string, any> = {};
    simpleBlock: ISimpleCityBlock;
    jobInfoMap: Record<string, T> = {};
    jobDependencies: Record<string, string[]> = {};
    loadJobInfoHelper: (ctx: PSCityBlock<T>, jobId: string) => Promise<T>;

    constructor(
        rpc: ICityRPCProvider,
        dogeRPC: IDogeLinkElectrsRPC,
        synthResult: ICitySynthBlockResult,
        dumpConfig: IDumpProofStoreConfig,
        simpleBlock: ISimpleCityBlock,
        loadJobInfoHelper: (ctx: PSCityBlock<T>, jobId: string) => Promise<T>,
        blockExplorerUrl = "http://localhost:1337/explorer"
    ) {
        this.checkpoint_id = dumpConfig.checkpoint_id;
        this.rpc_node_id = dumpConfig.rpc_node_id;
        this.job_config = dumpConfig.job_config;
        this.rpc = new CityRPCProviderWithCache(rpc);
        this.dogeRPC = dogeRPC;
        this.synthResult = normalizeSynth(synthResult);
        this.simpleBlock = simpleBlock;
        this.blockExplorerUrl = blockExplorerUrl;
        this.loadJobInfoHelper = loadJobInfoHelper;
    }

    getSimpleCityBlock(): ISimpleCityBlock {
        const synth = synthPlanner({
            checkpoint_id: this.checkpoint_id,
            job_config: { ...this.job_config },
        });
        return {
            stateTransitionRoot: depSerializedToProofNodes(synth.root_state_transition),
            sighashProofs: synth.sighash_proofs,
        };
    }
    async loadJobWitness(jobId: string): Promise<ICityJobWitness> {
        if (Object.hasOwnProperty.call(this.jobIdWitnessCache, jobId)) {
            return this.jobIdWitnessCache[jobId];
        }
        const value = await this.rpc.getProofStoreJobWitness(jobId);
        this.jobIdWitnessCache[jobId] = value;
        return value;
    }
    async loadJobInfo(jobId: string): Promise<T> {
        if (Object.hasOwnProperty.call(this.jobInfoMap, jobId)) {
            return this.jobInfoMap[jobId];
        }
        const value = await this.loadJobInfoHelper(this, jobId);
        this.jobInfoMap[jobId] = value;
        return value;
    }
    getDependencyJobs(jobId: string): string[] {
        jobId = getJobWitnessIdHex(jobId);
        if (Object.hasOwnProperty.call(this.jobDependencies, jobId)) {
            return this.jobDependencies[jobId];
        }

        const decoded = deserializeJobId(jobId);
        if (decoded.circuit_type === ProvingJobCircuitType.WrapFinalSigHashProofBLS12381) {
            const found = this.synthResult.sighash_proofs.find((x) => x.groth16_final === jobId);
            if (found) {
                const deps = [found.sighash_final].concat(this.getDependencyJobs(found.sighash_final));
                this.jobDependencies[jobId] = deps;
                return deps;
            } else {
                throw new Error("Could not find job " + jobId + " in sighash_proofs");
            }
            //return this.synthResult.sighash_proofs.find(x=>)
        } else if (decoded.circuit_type === ProvingJobCircuitType.GenerateFinalSigHashProof) {
            const found = this.synthResult.sighash_proofs.find((x) => x.sighash_final === jobId);
            if (found) {
                const deps = [found.sighash_introspection, found.state_transition_reference].concat(
                    this.getDependencyJobs(found.state_transition_reference)
                );
                this.jobDependencies[jobId] = deps;
                return deps;
            } else {
                throw new Error("Could not find job " + jobId + " in sighash_proofs");
            }
        } else if (decoded.circuit_type === ProvingJobCircuitType.GenerateSigHashIntrospectionProof) {
            return [];
        } else {
            const found = findInRST(jobId, this.synthResult.root_state_transition);
            if (!found) {
                throw new Error("Could not find job " + jobId + " in root_state_transition");
            }
            const deps = getFlatDependenciesForRST(found);
            this.jobDependencies[jobId] = deps;
            return deps;
        }
    }
    getDirectDependencyJobs(jobId: string): string[] {
        jobId = getJobWitnessIdHex(jobId);

        const decoded = deserializeJobId(jobId);
        if (decoded.circuit_type === ProvingJobCircuitType.WrapFinalSigHashProofBLS12381) {
            const found = this.synthResult.sighash_proofs.find((x) => x.groth16_final === jobId);
            if (found) {
                return [found.sighash_final];
            } else {
                throw new Error("Could not find job " + jobId + " in sighash_proofs");
            }
            //return this.synthResult.sighash_proofs.find(x=>)
        } else if (decoded.circuit_type === ProvingJobCircuitType.GenerateFinalSigHashProof) {
            const found = this.synthResult.sighash_proofs.find((x) => x.sighash_final === jobId);
            if (found) {
                return [found.sighash_introspection, found.state_transition_reference];
            } else {
                throw new Error("Could not find job " + jobId + " in sighash_proofs");
            }
        } else if (decoded.circuit_type === ProvingJobCircuitType.GenerateSigHashIntrospectionProof) {
            return [];
        } else {
            const found = findInRST(jobId, this.synthResult.root_state_transition);
            if (!found) {
                throw new Error("Could not find job " + jobId + " in root_state_transition");
            }
            return found.dependencies.map((x) => x.id);
        }
    }
}

export { PSCityBlock };
