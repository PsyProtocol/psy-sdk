import { WebProverConfig } from "./config";
import initWasm, { WasmRpcServer } from "./psy_prover";
import { PsyUserProverRPCCommand } from "../local-prover-rpc/types";
import { PsyJSON } from "../utils";

// Server-side message types
interface ServerRequest {
    type: 'request';
    id: string;
    clientId: string;
    method: string;
    params: any[];
    timestamp: number;
}

interface ServerResponse {
    type: 'response';
    id: string;
    clientId: string;
    result?: any;
    error?: string;
    timestamp: number;
}

interface ClientConnection {
    id: string;
    connectedAt: number;
    lastActivity: number;
}

interface InitMessage {
    type: 'init';
    config: WebProverConfig;
}

interface ClientRegisterMessage {
    type: 'register-client';
    clientId: string;
}

interface ClientUnregisterMessage {
    type: 'unregister-client';
    clientId: string;
}

// QED Prover Server implementation
class PsyProverServer {
    private wasmServer: WasmRpcServer | null = null;
    private isInitialized = false;
    private clients: Map<string, ClientConnection> = new Map();
    private stats = {
        totalRequests: 0,
        activeRequests: 0,
        errors: 0,
        startTime: Date.now()
    };

    async initialize(config: WebProverConfig): Promise<void> {
        if (this.isInitialized) return;
        
        try {
            await initWasm();
            const configJson = PsyJSON.stringify(config);
            this.wasmServer = new WasmRpcServer(configJson);
            this.isInitialized = true;
            console.log('QED Prover Server initialized successfully');
        } catch (error) {
            console.error('Failed to initialize QED Prover Server:', error);
            throw error;
        }
    }

    registerClient(clientId: string): void {
        this.clients.set(clientId, {
            id: clientId,
            connectedAt: Date.now(),
            lastActivity: Date.now()
        });
        console.log(`Client ${clientId} connected. Total clients: ${this.clients.size}`);
    }

    unregisterClient(clientId: string): void {
        this.clients.delete(clientId);
        console.log(`Client ${clientId} disconnected. Total clients: ${this.clients.size}`);
    }

    updateClientActivity(clientId: string): void {
        const client = this.clients.get(clientId);
        if (client) {
            client.lastActivity = Date.now();
        }
    }

    async handleRequest(request: ServerRequest): Promise<ServerResponse> {
        const { id, clientId, method, params } = request;
        
        this.stats.totalRequests++;
        this.stats.activeRequests++;
        this.updateClientActivity(clientId);

        try {
            if (!this.isInitialized || !this.wasmServer) {
                throw new Error('Server not initialized');
            }

            const result = await this.executeMethod(method, params);
            
            return {
                type: 'response',
                id,
                clientId,
                result,
                timestamp: Date.now()
            };
        } catch (error) {
            this.stats.errors++;
            return {
                type: 'response',
                id,
                clientId,
                error: error instanceof Error ? error.message : 'Unknown error',
                timestamp: Date.now()
            };
        } finally {
            this.stats.activeRequests--;
        }
    }

    private async executeMethod(method: string, params: any[]): Promise<any> {
        if (!this.wasmServer) {
            throw new Error('WASM server not available');
        }

        switch (method) {
            case PsyUserProverRPCCommand.ExecContractCall:
                return this.wasmServer.exec_contract_call_json(params[0], PsyJSON.stringify(params[1]));
            
            case PsyUserProverRPCCommand.StartSession:
                return this.wasmServer.start_session(params[0]);
            
            case PsyUserProverRPCCommand.ProveContractCall:
                return this.wasmServer.prove_contract_call_json(params[0], PsyJSON.stringify(params[1]));
            
            case PsyUserProverRPCCommand.ProveContractCalls:
                return this.wasmServer.prove_contract_calls_json(params[0], PsyJSON.stringify(params[1]));
            
            case PsyUserProverRPCCommand.SignAndSubmit:
                return this.wasmServer.sign_and_submit(params[0]);
            
            case PsyUserProverRPCCommand.RegisterUser:
                return this.wasmServer.register_user(params[0]);
            
            case PsyUserProverRPCCommand.AddUser:
                return this.wasmServer.add_user(params[0]);
            
            case PsyUserProverRPCCommand.GetZKPublicKey: {
                const zkResult = await this.wasmServer.get_zk_public_key_json(params[0]);
                return PsyJSON.parse(zkResult);
            }
            
            case PsyUserProverRPCCommand.GetRandomKeypair: {
                const keypairResult = await this.wasmServer.get_random_keypair_json();
                return PsyJSON.parse(keypairResult);
            }
            
            case PsyUserProverRPCCommand.DeployContract:
                return this.wasmServer.deploy_contract_json(params[0], PsyJSON.stringify(params[1]));
            
            case PsyUserProverRPCCommand.GetDeployContractCmd: {
                const deployResult = await this.wasmServer.get_deploy_contract_cmd_json(params[0], PsyJSON.stringify(params[1]));
                return PsyJSON.parse(deployResult);
            }
            
            case PsyUserProverRPCCommand.Ping:
                return this.wasmServer.ping(params[0]);
            
            case PsyUserProverRPCCommand.GetResult:
                return this.wasmServer.get_result(params[0]);
            
            case 'getServerStats':
                return this.getServerStats();
            
            default:
                throw new Error(`Unknown method: ${method}`);
        }
    }

    getServerStats() {
        return {
            ...this.stats,
            connectedClients: this.clients.size,
            uptime: Date.now() - this.stats.startTime,
            clients: Array.from(this.clients.values())
        };
    }
}

// Global server instance
const proverServer = new PsyProverServer();

// Message handler
self.onmessage = async (event: MessageEvent) => {
    const { data } = event;
    
    // Handle server initialization
    if (data.type === 'init') {
        try {
            await proverServer.initialize(data.config);
            self.postMessage({
                type: 'init-response',
                success: true,
                message: 'Server initialized successfully'
            });
        } catch (error) {
            self.postMessage({
                type: 'init-response',
                success: false,
                error: error instanceof Error ? error.message : 'Unknown error'
            });
        }
        return;
    }

    // Handle client registration
    if (data.type === 'register-client') {
        proverServer.registerClient(data.clientId);
        self.postMessage({
            type: 'client-registered',
            clientId: data.clientId
        });
        return;
    }

    // Handle client unregistration
    if (data.type === 'unregister-client') {
        proverServer.unregisterClient(data.clientId);
        self.postMessage({
            type: 'client-unregistered',
            clientId: data.clientId
        });
        return;
    }

    // Handle method requests
    if (data.type === 'request') {
        const response = await proverServer.handleRequest(data as ServerRequest);
        self.postMessage(response);
        return;
    }
};

// Export types
export type { 
    ServerRequest, 
    ServerResponse, 
    ClientConnection,
    InitMessage,
    ClientRegisterMessage,
    ClientUnregisterMessage
}; 