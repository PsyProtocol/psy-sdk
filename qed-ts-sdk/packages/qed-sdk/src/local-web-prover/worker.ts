import { WebProverConfig } from "./config";
import initWasm, { WasmRpcServer } from "./qed_user_prover";
import { QedUserProverRPCCommand } from "../local-prover-rpc/types";
import { QedJSON } from "../utils";

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

// Worker state
let wasmServer: WasmRpcServer | null = null;
let isInitialized = false;

// Initialize WASM and create server
async function initializeWasm(config: WebProverConfig): Promise<void> {
    if (isInitialized) return;
    
    try {
        await initWasm();
        const configJson = QedJSON.stringify(config);
        wasmServer = new WasmRpcServer(configJson);
        isInitialized = true;
        console.log('WASM Prover initialized successfully');
    } catch (error) {
        console.error('Failed to initialize WASM prover:', error);
        throw error;
    }
}

// Handle method calls
async function handleMethodCall(method: string, params: any[]): Promise<any> {
    if (!wasmServer) {
        throw new Error('WASM server not initialized');
    }

    switch (method) {
        case QedUserProverRPCCommand.ExecContractCall:
            return wasmServer.exec_contract_call_json(params[0], QedJSON.stringify(params[1]));
        
        case QedUserProverRPCCommand.StartSession:
            return wasmServer.start_session(params[0]);
        
        case QedUserProverRPCCommand.ProveContractCall:
            return wasmServer.prove_contract_call_json(params[0], QedJSON.stringify(params[1]));
        
        case QedUserProverRPCCommand.ProveContractCalls:
            return wasmServer.prove_contract_calls_json(params[0], QedJSON.stringify(params[1]));
        
        case QedUserProverRPCCommand.SignAndSubmit:
            return wasmServer.sign_and_submit(params[0]);
        
        case QedUserProverRPCCommand.RegisterUser:
            return wasmServer.register_user(params[0]);
        
        case QedUserProverRPCCommand.AddUser:
            return wasmServer.add_user(params[0]);
        
        case QedUserProverRPCCommand.GetZKPublicKey: {
            const zkResult = await wasmServer.get_zk_public_key_json(params[0]);
            return QedJSON.parse(zkResult);
        }
        
        case QedUserProverRPCCommand.GetRandomKeypair: {
            const keypairResult = await wasmServer.get_random_keypair_json();
            return QedJSON.parse(keypairResult);
        }
        
        case QedUserProverRPCCommand.DeployContract:
            return wasmServer.deploy_contract_json(params[0], QedJSON.stringify(params[1]));
        
        case QedUserProverRPCCommand.GetDeployContractCmd: {
            const deployResult = await wasmServer.get_deploy_contract_cmd_json(params[0], QedJSON.stringify(params[1]));
            return QedJSON.parse(deployResult);
        }
        
        case QedUserProverRPCCommand.Ping:
            return wasmServer.ping(params[0]);
        
        case QedUserProverRPCCommand.GetResult:
            return wasmServer.get_result(params[0]);
        
        default:
            throw new Error(`Unknown method: ${method}`);
    }
}

// Main message handler
self.onmessage = async (event: MessageEvent<WorkerMessage | { method: 'init'; config: WebProverConfig }>) => {
    const { data } = event;
    
    // Handle initialization
    if ('method' in data && data.method === 'init') {
        try {
            const initData = data as { method: 'init'; config: WebProverConfig };
            await initializeWasm(initData.config);
            self.postMessage({ 
                id: 'init', 
                result: 'initialized' 
            } as WorkerResponse);
        } catch (error) {
            self.postMessage({ 
                id: 'init', 
                error: error instanceof Error ? error.message : 'Unknown error' 
            } as WorkerResponse);
        }
        return;
    }

    // Handle regular method calls
    if ('id' in data && 'method' in data) {
        try {
            const result = await handleMethodCall(data.method, data.params);
            self.postMessage({
                id: data.id,
                result
            } as WorkerResponse);
        } catch (error) {
            self.postMessage({
                id: data.id,
                error: error instanceof Error ? error.message : 'Unknown error'
            } as WorkerResponse);
        }
    }
};

// Export types for main thread
export type { WorkerMessage, WorkerResponse }; 