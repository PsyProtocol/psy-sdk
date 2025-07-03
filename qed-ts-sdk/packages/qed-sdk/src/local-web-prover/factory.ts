import { WebProverConfig, createDefaultRpcConfig } from "./config";
import { QedWasmWebProverProvider } from "./provider";
import { QedProverClient } from "./workerProvider";
import { IQedUserProverProvider } from "../local-prover-rpc/types";

export interface ProverFactoryOptions {
    useWorker?: boolean;
    workerScript?: string;
    config?: WebProverConfig;
}


export function createQedProverProvider(
    options: ProverFactoryOptions = {}
): IQedUserProverProvider {
    const {
        useWorker = false,
        workerScript = './worker.js', // Default worker script path
        config = createDefaultRpcConfig()
    } = options;

    if (useWorker) {
        if (typeof Worker === 'undefined') {
            console.warn('Web Worker not supported in this environment, falling back to main thread provider');
            return new QedWasmWebProverProvider(config);
        }
        
        return new QedProverClient(workerScript, config);
    } else {
        return new QedWasmWebProverProvider(config);
    }
}

export function createMainThreadProvider(config?: WebProverConfig): QedWasmWebProverProvider {
    return new QedWasmWebProverProvider(config || createDefaultRpcConfig());
}


export function createWebWorkerProvider(
    workerScript: string,
    config?: WebProverConfig
): QedProverClient {
    return new QedProverClient(workerScript, config || createDefaultRpcConfig());
} 