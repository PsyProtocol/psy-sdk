import { WebProverConfig } from "./config";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQedUserProverProvider,
    QBCDeployContract,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";

// Message types for worker communication
interface WorkerMessage {
    id: string;
    method: string;
    params: any[];
}

interface WorkerResponse {
    id: string;
    result?: any;
    error?: string;
}

export class QedWasmWebWorkerProverProvider implements IQedUserProverProvider {
    private worker: Worker;
    private pendingRequests: Map<string, { resolve: (value: any) => void; reject: (error: Error) => void }>;
    private requestId: number;
    private isInitialized: boolean;

    constructor(workerScript: string, rpcConfigJson: WebProverConfig) {
        this.worker = new Worker(workerScript, { type: 'module' });
        this.pendingRequests = new Map();
        this.requestId = 0;
        this.isInitialized = false;

        // Set up message handler
        this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
            this.handleWorkerMessage(event.data);
        };

        this.worker.onerror = (error) => {
            console.error('Worker error:', error);
        };

        // Initialize the worker
        this.initializeWorker(rpcConfigJson);
    }

    private async initializeWorker(config: WebProverConfig): Promise<void> {
        return new Promise((resolve, reject) => {
            this.worker.postMessage({ method: 'init', config });
            
            const handleInitResponse = (event: MessageEvent<WorkerResponse>) => {
                if (event.data.id === 'init') {
                    this.worker.removeEventListener('message', handleInitResponse);
                    if (event.data.error) {
                        reject(new Error(event.data.error));
                    } else {
                        this.isInitialized = true;
                        resolve();
                    }
                }
            };
            
            this.worker.addEventListener('message', handleInitResponse);
        });
    }

    private handleWorkerMessage(response: WorkerResponse): void {
        const request = this.pendingRequests.get(response.id);
        if (request) {
            this.pendingRequests.delete(response.id);
            if (response.error) {
                request.reject(new Error(response.error));
            } else {
                request.resolve(response.result);
            }
        }
    }

    private async callWorkerMethod(method: string, params: any[]): Promise<any> {
        if (!this.isInitialized) {
            throw new Error('Worker not initialized');
        }

        const id = (++this.requestId).toString();
        const message: WorkerMessage = { id, method, params };

        return new Promise((resolve, reject) => {
            this.pendingRequests.set(id, { resolve, reject });
            this.worker.postMessage(message);
        });
    }

    async execContractCall(pkHash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
        return this.callWorkerMethod('execContractCall', [pkHash, contractCallArg]);
    }

    // Local proving operations
    async startSession(pkHash: PublicKey): Promise<string> {
        return this.callWorkerMethod('startSession', [pkHash]);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        return this.callWorkerMethod('proveContractCall', [pkHash, contractCallArg]);
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        return this.callWorkerMethod('proveContractCalls', [pkHash, contractCallArgs]);
    }

    async signAndSubmit(pkHash: PublicKey): Promise<string> {
        return this.callWorkerMethod('signAndSubmit', [pkHash]);
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.callWorkerMethod('registerUser', [privateKey.toString()]);
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.callWorkerMethod('addUser', [privateKey.toString()]);
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        return this.callWorkerMethod('getZKPublicKey', [privateKey.toString()]);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.callWorkerMethod('getRandomKeypair', []);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.callWorkerMethod('deployContract', [deployer, circuitDefs]);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        return this.callWorkerMethod('getDeployContractCmd', [deployer, circuitDefs]);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return this.callWorkerMethod('ping', [message]);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.callWorkerMethod('getResult', [id.toString()]);
    }

    // Clean up resources
    terminate(): void {
        this.worker.terminate();
        // Reject all pending requests
        for (const request of this.pendingRequests.values()) {
            request.reject(new Error('Worker terminated'));
        }
        this.pendingRequests.clear();
    }
} 