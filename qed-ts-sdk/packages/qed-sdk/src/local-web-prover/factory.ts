import { WebProverConfig, createDefaultRpcConfig } from "./config";
import { QedWasmWebProverProvider } from "./provider";
import { QedWasmWebWorkerProverProvider } from "./workerProvider";
import { IQedUserProverProvider } from "../local-prover-rpc/types";

export interface ProverFactoryOptions {
    useWorker?: boolean;
    workerScript?: string;
    config?: WebProverConfig;
}


export async function createQedProverProvider(
    options: ProverFactoryOptions = {}
): Promise<IQedUserProverProvider> {
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
        
        const workerProvider = new QedWasmWebWorkerProverProvider(workerScript, config);
        // Wait for worker initialization
        await new Promise((resolve) => {
            // The worker provider handles initialization internally
            // This is just to ensure the API is consistent
            setTimeout(resolve, 100);
        });
        return workerProvider;
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
): QedWasmWebWorkerProverProvider {
    return new QedWasmWebWorkerProverProvider(workerScript, config || createDefaultRpcConfig());
} 