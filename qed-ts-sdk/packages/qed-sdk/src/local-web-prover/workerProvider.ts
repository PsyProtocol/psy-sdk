import { WebProverConfig } from "./config";
import { ServerRequest, ServerResponse } from "./worker";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQedUserProverProvider,
    QedUserProverRPCCommand,
    QBCDeployContract,
    WalletKeyPair,
    SignData,
} from "../local-prover-rpc/types";
import { JobInfo, ZKPublicKeyInfo } from "../types";
import { QedJSON } from "../utils";

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
class QedWorkerManager {
    private static instance: QedWorkerManager | null = null;
    private worker: Worker | null = null;
    private clients: Map<string, QedProverClient> = new Map();
    private isInitialized = false;
    private initPromise: Promise<void> | null = null;

    private constructor() {}

    static getInstance(): QedWorkerManager {
        if (!QedWorkerManager.instance) {
            QedWorkerManager.instance = new QedWorkerManager();
        }
        return QedWorkerManager.instance;
    }

    async initializeWorker(workerScript: string, config: WebProverConfig): Promise<void> {
        if (this.isInitialized) return;

        if (this.initPromise) {
            return this.initPromise;
        }

        this.initPromise = this.doInitialize(workerScript, config);
        await this.initPromise;
    }

    private async doInitialize(workerScript: string, config: WebProverConfig): Promise<void> {
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

    private async initWorker(config: WebProverConfig): Promise<void> {
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

    registerClient(client: QedProverClient): void {
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
        QedWorkerManager.instance = null;
    }
}

/**
 * QED Prover Client - connects to Worker Server
 */
export class QedProverClient implements IQedUserProverProvider {
    private static workerManager = QedWorkerManager.getInstance();
    
    private clientId: string;
    private pendingRequests: Map<string, ClientRequest> = new Map();
    private requestCounter = 0;
    private isConnected = false;

    constructor(workerScript: string, config: WebProverConfig) {
        this.clientId = this.generateClientId();
        this.initialize(workerScript, config);
    }

    private async initialize(workerScript: string, config: WebProverConfig): Promise<void> {
        try {
            // Ensure Worker is initialized
            await QedProverClient.workerManager.initializeWorker(workerScript, config);
            
            // Register current client
            QedProverClient.workerManager.registerClient(this);
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

            QedProverClient.workerManager.sendRequest(this.clientId, serverRequest);
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

    // IQedUserProverProvider implementation
    async execContractCall(pkHash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
        return this.callServerMethod(QedUserProverRPCCommand.ExecContractCall, [pkHash, contractCallArg]);
    }

    async execContractCallWithSignData(pkHash: string, contractCallArg: ContractCallArgs[], signData: SignData|null|undefined): Promise<QHashOut> {
        const signDataJson = signData ? QedJSON.stringify(signData) : null;
        return this.callServerMethod(QedUserProverRPCCommand.ExecContractCallWithSignData, [pkHash, contractCallArg, signDataJson]);
    }

    async startSession(pkHash: PublicKey): Promise<string> {
        return this.callServerMethod(QedUserProverRPCCommand.StartSession, [pkHash]);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        return this.callServerMethod(QedUserProverRPCCommand.ProveContractCall, [pkHash, contractCallArg]);
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        return this.callServerMethod(QedUserProverRPCCommand.ProveContractCalls, [pkHash, contractCallArgs]);
    }

    async signAndSubmit(pkHash: PublicKey): Promise<string> {
        return this.callServerMethod(QedUserProverRPCCommand.SignAndSubmit, [pkHash]);
    }

    async signAndSubmitWithData(pkHash: PublicKey, signData: SignData|null|undefined): Promise<string> {
        const signDataJson = signData ? QedJSON.stringify(signData) : null;
        return this.callServerMethod(QedUserProverRPCCommand.SignAndSubmitWithData, [pkHash, signDataJson]);
    }

    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.callServerMethod(QedUserProverRPCCommand.RegisterUser, [privateKey]);
    }

    async registerUserWithType(privateKey: PrivateKey, signType: string, fingerprint: string|null|undefined): Promise<PublicKey> {
        return this.callServerMethod(QedUserProverRPCCommand.RegisterUserWithType, [privateKey, signType, fingerprint]);
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.callServerMethod(QedUserProverRPCCommand.AddUser, [privateKey]);
    }

    async addUserWithType(privateKey: PrivateKey, signType: string, fingerprint: string|null|undefined): Promise<PublicKey> {
        return this.callServerMethod(QedUserProverRPCCommand.AddUserWithType, [privateKey, signType, fingerprint]);
    }

    async getClaimRewardsCallArgs(pkHash: PublicKey, jobInfos: string): Promise<ContractCallArgs[]> {
        throw new Error("Method not implemented.");
    }

    async claimRewards(pkHash: PublicKey, jobInfos: string): Promise<string> {
        throw new Error("Method not implemented.");
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        return this.callServerMethod(QedUserProverRPCCommand.GetZKPublicKey, [privateKey]);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.callServerMethod(QedUserProverRPCCommand.GetRandomKeypair, []);
    }

    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.callServerMethod(QedUserProverRPCCommand.DeployContract, [deployer, circuitDefs]);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        return this.callServerMethod(QedUserProverRPCCommand.GetDeployContractCmd, [deployer, circuitDefs]);
    }

    async ping(message: string): Promise<string> {
        return this.callServerMethod(QedUserProverRPCCommand.Ping, [message]);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.callServerMethod(QedUserProverRPCCommand.GetResult, [id]);
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
            QedProverClient.workerManager.unregisterClient(this.clientId);
            
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
        QedWorkerManager.getInstance().terminate();
    }
    
    static getGlobalStats() {
        return QedWorkerManager.getInstance().getStats();
    }
} 