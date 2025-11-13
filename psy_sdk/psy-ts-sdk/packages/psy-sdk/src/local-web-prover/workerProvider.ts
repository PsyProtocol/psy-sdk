import { ServerRequest, ServerResponse } from "./worker";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    ContractCallData,
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    PsyUserProverRPCCommand,
    QBCDeployContract,
    SignData,
    SignType,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { PsyNetworkConfig } from "../config";

// Client-side types
interface ClientRequest {
    id: string;
    method: string;
    params: any[];
    resolve: (value: any) => void;
    reject: (error: Error) => void;
    timeout: NodeJS.Timeout;
}

interface ServerStats {
    totalRequests: number;
    activeRequests: number;
    errors: number;
    uptime: number;
    connectedClients: number;
    clients: Array<{
        id: string;
        connectedAt: number;
        lastActivity: number;
    }>;
}



/**
 * Global Worker Manager - Singleton pattern
 * Manages a single worker instance shared by multiple providers
 */
class PsyWorkerManager {
    private static instance: PsyWorkerManager | null = null;
    private worker: Worker | null = null;
    private clients: Map<string, PsyProverClient> = new Map();
    private isInitialized = false;
    private initPromise: Promise<void> | null = null;

    private constructor() {}

    static getInstance(): PsyWorkerManager {
        if (!PsyWorkerManager.instance) {
            PsyWorkerManager.instance = new PsyWorkerManager();
        }
        return PsyWorkerManager.instance;
    }

    async initializeWorker(workerScript: string, config: PsyNetworkConfig): Promise<void> {
        if (this.isInitialized) return;

        if (this.initPromise) {
            return this.initPromise;
        }

        this.initPromise = this.doInitialize(workerScript, config);
        await this.initPromise;
    }

    private async doInitialize(workerScript: string, config: PsyNetworkConfig): Promise<void> {
        if (typeof Worker === 'undefined') {
            throw new Error('Web Workers are not supported in this environment');
        }

        this.worker = new Worker(workerScript, { type: 'module' });

        // Only one message listener for the entire manager
        this.worker.onmessage = (event: MessageEvent) => {
            this.handleWorkerMessage(event.data);
        };

        this.worker.onerror = (error) => {
            console.error('Worker error:', error);
            this.notifyAllClients('error', error);
        };

        // Initialize worker with config
        await this.initWorker(config);
        this.isInitialized = true;
    }

    private async initWorker(config: PsyNetworkConfig): Promise<void> {
        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new Error('Worker initialization timeout'));
            }, 10000);

            const handleInitResponse = (event: MessageEvent) => {
                if (event.data.type === 'init-response') {
                    clearTimeout(timeout);
                    this.worker!.removeEventListener('message', handleInitResponse);

                    if (event.data.success) {
                        resolve();
                    } else {
                        reject(new Error(event.data.error));
                    }
                }
            };

            this.worker!.addEventListener('message', handleInitResponse);
            this.worker!.postMessage({
                type: 'init',
                config
            });
        });
    }

    registerClient(client: PsyProverClient): void {
        this.clients.set(client.getClientId(), client);
        console.log(`Client registered: ${client.getClientId()}, Total: ${this.clients.size}`);

        // Register client with worker
        if (this.worker && this.isInitialized) {
            this.worker.postMessage({
                type: 'register-client',
                clientId: client.getClientId()
            });
        }
    }

    unregisterClient(clientId: string): void {
        this.clients.delete(clientId);
        console.log(`Client unregistered: ${clientId}, Total: ${this.clients.size}`);

        // Unregister client with worker
        if (this.worker && this.isInitialized) {
            this.worker.postMessage({
                type: 'unregister-client',
                clientId: clientId
            });
        }
    }

    async sendRequest(_clientId: string, request: ServerRequest): Promise<void> {
        if (!this.worker || !this.isInitialized) {
            throw new Error('Worker not initialized');
        }
        this.worker.postMessage(request);
    }

    private handleWorkerMessage(data: any): void {
        if (data.type === 'response') {
            // Route message to specific client
            const targetClient = this.clients.get(data.clientId);
            if (targetClient) {
                targetClient.handleResponse(data);
            } else {
                console.warn(`Response for unknown client: ${data.clientId}`);
            }
        } else if (data.type === 'broadcast') {
            // Broadcast message to all clients
            this.notifyAllClients('broadcast', data);
        } else if (data.type === 'client-registered' || data.type === 'client-unregistered') {
            // Handle client registration confirmations
            console.log('Worker confirmed:', data);
        }
    }

    private notifyAllClients(event: string, data: any): void {
        for (const client of this.clients.values()) {
            client.handleEvent(event, data);
        }
    }

    getStats() {
        return {
            totalClients: this.clients.size,
            isInitialized: this.isInitialized,
            clientIds: Array.from(this.clients.keys())
        };
    }

    terminate(): void {
        if (this.worker) {
            this.worker.terminate();
            this.worker = null;
        }

        this.notifyAllClients('terminated', null);
        this.clients.clear();
        this.isInitialized = false;
        this.initPromise = null;
        PsyWorkerManager.instance = null;
    }
}

/**
 * Psy Prover Client - connects to Worker Server
 */
export class PsyProverClient implements IPsyUserProverProvider {
    private static workerManager = PsyWorkerManager.getInstance();

    private clientId: string;
    private pendingRequests: Map<string, ClientRequest> = new Map();
    private requestCounter = 0;
    private isConnected = false;

    constructor(workerScript: string, config: PsyNetworkConfig) {
        this.clientId = this.generateClientId();
        this.initialize(workerScript, config);
    }

    private async initialize(workerScript: string, config: PsyNetworkConfig): Promise<void> {
        try {
            // Ensure Worker is initialized
            await PsyProverClient.workerManager.initializeWorker(workerScript, config);

            // Register current client
            PsyProverClient.workerManager.registerClient(this);
            this.isConnected = true;

            console.log(`Client ${this.clientId} connected to server`);
        } catch (error) {
            console.error('Failed to initialize client:', error);
            throw error;
        }
    }

    // Only manager calls this method
    handleResponse(data: ServerResponse): void {
        const request = this.pendingRequests.get(data.id);
        if (request) {
            clearTimeout(request.timeout);
            this.pendingRequests.delete(data.id);

            if (data.error) {
                request.reject(new Error(data.error));
            } else {
                request.resolve(data.result);
            }
        }
    }

    // Handle broadcast events
    handleEvent(event: string, data: any): void {
        console.log(`Client ${this.clientId} received event: ${event}`, data);
        // Can handle server broadcast events here
    }

    private async callServerMethod(method: string, params: any[], timeoutMs = 30000): Promise<any> {
        if (!this.isConnected) {
            throw new Error('Client not connected');
        }

        const requestId = this.generateRequestId();

        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                this.pendingRequests.delete(requestId);
                reject(new Error(`Request ${requestId} timeout after ${timeoutMs}ms`));
            }, timeoutMs);

            const request: ClientRequest = {
                id: requestId,
                method,
                params,
                resolve,
                reject,
                timeout
            };

            this.pendingRequests.set(requestId, request);

            // Send through manager
            const serverRequest: ServerRequest = {
                type: 'request',
                id: requestId,
                clientId: this.clientId,
                method,
                params,
                timestamp: Date.now()
            };

            PsyProverClient.workerManager.sendRequest(this.clientId, serverRequest);
        });
    }

    private generateClientId(): string {
        return `client_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }

    private generateRequestId(): string {
        return `${this.clientId}_${++this.requestCounter}`;
    }

    getClientId(): string {
        return this.clientId;
    }

    // IPsyUserProverProvider implementation
    async execContractCall(pkHash: string, callData: ContractCallData): Promise<string> {
        return this.callServerMethod(PsyUserProverRPCCommand.ExecContractCall, [pkHash, callData]);
    }

    async startSession(pkHash: PublicKey): Promise<string> {
        return this.callServerMethod(PsyUserProverRPCCommand.StartSession, [pkHash]);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        return this.callServerMethod(PsyUserProverRPCCommand.ProveContractCall, [pkHash, contractCallArg]);
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        return this.callServerMethod(PsyUserProverRPCCommand.ProveContractCalls, [pkHash, contractCallArgs]);
    }

    async signAndSubmit(pkHash: PublicKey, signData?: SignData): Promise<string> {
        return this.callServerMethod(PsyUserProverRPCCommand.SignAndSubmit, [pkHash, signData]);
    }

    async registerUser(privateKey: PrivateKey, signType: SignType): Promise<PublicKey> {
        return this.callServerMethod(PsyUserProverRPCCommand.RegisterUser, [privateKey, signType]);
    }

    async addUser(privateKey: PrivateKey, signType: SignType): Promise<PublicKey> {
        return this.callServerMethod(PsyUserProverRPCCommand.AddUser, [privateKey, signType]);
    }

    async getClaimRewardsCallArgs(_jobInfos: string): Promise<ContractCallArgs[]> {
        throw new Error("Method not implemented.");
    }

    async claimRewards(_pkHash: PublicKey, _jobInfos: string): Promise<string> {
        throw new Error("Method not implemented.");
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        return this.callServerMethod(PsyUserProverRPCCommand.GetZKPublicKey, [privateKey]);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.callServerMethod(PsyUserProverRPCCommand.GetRandomKeypair, []);
    }

    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.callServerMethod(PsyUserProverRPCCommand.DeployContract, [deployer, circuitDefs]);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        return this.callServerMethod(PsyUserProverRPCCommand.GetDeployContractCmd, [deployer, circuitDefs]);
    }

    async ping(message: string): Promise<string> {
        return this.callServerMethod(PsyUserProverRPCCommand.Ping, [message]);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.callServerMethod(PsyUserProverRPCCommand.GetResult, [id]);
    }

    // Client-specific methods
    async getServerStats(): Promise<ServerStats> {
        return this.callServerMethod('getServerStats', []);
    }

    getClientInfo() {
        return {
            clientId: this.clientId,
            isConnected: this.isConnected,
            pendingRequests: this.pendingRequests.size
        };
    }

    disconnect(): void {
        if (this.isConnected) {
            PsyProverClient.workerManager.unregisterClient(this.clientId);

            // Clear pending requests
            for (const request of this.pendingRequests.values()) {
                clearTimeout(request.timeout);
                request.reject(new Error('Client disconnected'));
            }
            this.pendingRequests.clear();
            this.isConnected = false;
        }
    }

    static terminateServer(): void {
        PsyWorkerManager.getInstance().terminate();
    }

    static getGlobalStats() {
        return PsyWorkerManager.getInstance().getStats();
    }
}
