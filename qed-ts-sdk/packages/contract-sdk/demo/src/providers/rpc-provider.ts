// src/providers/rpc-provider.ts
import axios, { AxiosInstance } from 'axios';
import { IContractProvider, GUint } from '../sdk/types';
import { config, getRpcUrls, getRealmRpcUrl, getCoordinatorRpcUrl, getKeyPairByPublicKey } from '../config';
import { QedJSON } from '@qed/qed-sdk';

export interface RpcRequest {
    jsonrpc: '2.0';
    method: string;
    params: any;
    id: number | string;
}

export interface RpcResponse<T = any> {
    jsonrpc: '2.0';
    result?: T;
    error?: {
        code: number;
        message: string;
        data?: any;
    };
    id: number | string;
}

export interface MerkleProofResponse {
    root: string;
    value: string;
    index: number;
    siblings: string[];
}

export class RpcProvider implements IContractProvider {
    private realmClient: AxiosInstance;
    private coordinatorClient: AxiosInstance;
    private requestId: number = 1;

    // Store private keys for signing (in production, use secure key management)
    private privateKeys: Map<string, string> = new Map();

    constructor(
        private realmRpcUrl: string = config.rpc.url,
        private coordinatorRpcUrl: string = config.rpc.coordinatorUrl
    ) {
        // Client for Realm RPC (state queries)
        this.realmClient = axios.create({
            baseURL: realmRpcUrl,
            timeout: config.rpc.timeout,
            headers: {
                'Content-Type': 'application/json',
            },
        });

        // Client for Coordinator RPC (global operations)
        this.coordinatorClient = axios.create({
            baseURL: coordinatorRpcUrl,
            timeout: config.rpc.timeout,
            headers: {
                'Content-Type': 'application/json',
            },
        });

        // Initialize private keys (in production, this would use secure key management)
        this.initializeKeys();

        // Add request/response interceptors for logging
        if (config.logging.enabled) {
            this.setupLogging();
        }
    }

    private initializeKeys() {
        // Initialize with key pairs from config
        Object.values(config.user.keyPairs).forEach(keyPair => {
            this.privateKeys.set(keyPair.publicKey, keyPair.privateKey);
        });

        if (config.logging.logSignerOps) {
            console.log(`🔑 Initialized ${this.privateKeys.size} key pairs`);
        }
    }

    private derivePublicKey(privateKey: string): string {
        // Mock derivation - in production use proper cryptography
        return '0x' + privateKey.substring(0, 40);
    }

    private setupLogging() {
        // Setup logging for Realm client
        this.setupClientLogging(this.realmClient, 'Realm');

        // Setup logging for Coordinator client
        this.setupClientLogging(this.coordinatorClient, 'Coordinator');
    }

    private setupClientLogging(client: AxiosInstance, clientType: string) {
        // Request interceptor
        client.interceptors.request.use(
            (request) => {
                console.log(`\n📤 ${clientType} RPC Request to ${request.baseURL}:`);
                console.log(`   Method: ${request.data?.method}`);
                console.log(`   Params:`, request.data?.params);
                return request;
            },
            (error) => {
                console.error(`❌ ${clientType} Request error:`, error);
                return Promise.reject(error);
            }
        );

        // Response interceptor
        client.interceptors.response.use(
            (response) => {
                console.log(`\n📥 ${clientType} RPC Response:`);
                if (response.data.error) {
                    console.error(`   ❌ Error:`, response.data.error);
                } else {
                    console.log(`   ✅ Success:`, response.data.result);
                }
                return response;
            },
            (error) => {
                console.error(`❌ ${clientType} Response error:`, error.message);
                return Promise.reject(error);
            }
        );
    }

    // ========== REALM RPC METHODS (State Queries) ==========

    async getContractState(
        contractId: GUint,
        userId: GUint,
        offsets: GUint[]  // These should be offsets, not slots
    ): Promise<GUint[]> {
        try {
            const checkpointId = await this.getLatestCheckpoint();
            const contractStateHeight = config.contract.stateHeight;

            // Group offsets by their corresponding slot
            const offsetsBySlot = new Map<number, { index: number; offset: GUint; position: number }[]>();

            offsets.forEach((offset, index) => {
                // Convert offset to slot and position
                const slotIndex = Math.floor(Number(offset) / 4);
                const position = Number(offset) % 4;

                if (!offsetsBySlot.has(slotIndex)) {
                    offsetsBySlot.set(slotIndex, []);
                }

                offsetsBySlot.get(slotIndex)!.push({ index, offset, position });
            });

            // Initialize result array
            const stateValues: GUint[] = new Array(offsets.length);

            // Fetch each unique slot
            for (const [slotIndex, offsetInfos] of offsetsBySlot) {
                const request: RpcRequest = {
                    jsonrpc: '2.0',
                    method: 'qed_get_user_contract_state_tree_merkle_proof',
                    params: [
                        Number(checkpointId),
                        Number(userId),
                        Number(contractId),
                        contractStateHeight,
                        slotIndex  // Send the calculated slot index
                    ],
                    id: this.requestId++,
                };

                const response = await this.realmClient.post<RpcResponse<MerkleProofResponse>>('', request);

                if (response.data.error) {
                    throw new Error(`RPC Error: ${response.data.error.message}`);
                }

                if (!response.data.result) {
                    throw new Error('No result in RPC response');
                }

                // Extract the leaf value from the merkle proof response
                const leafValue = response.data.result.value;

                if (config.logging.enabled) {
                    console.log(`   Leaf value for slot ${slotIndex}: ${leafValue}`);
                }

                // Parse the leaf to get all 4 felts
                const felts = this.parseLeafValue(leafValue);

                // Assign the correct felt to each requested offset
                for (const { index, position } of offsetInfos) {
                    stateValues[index] = felts[position];

                    if (config.logging.enabled) {
                        console.log(`   Offset ${offsets[index]} → Slot ${slotIndex}, Position ${position}: ${felts[position]} (0x${felts[position].toString(16)})`);
                    }
                }
            }

            return stateValues;
        } catch (error) {
            if (axios.isAxiosError(error)) {
                throw new Error(`Network error: ${error.message}`);
            }
            throw error;
        }
    }

    /**
     * Parse a leaf value (256 bits) into 4 felts (64 bits each)
     * The leaf contains 4 consecutive felts in little-endian order within the leaf
     */
    private parseLeafValue(leafValue: string): GUint[] {
        // Remove '0x' prefix if present
        const cleanValue = leafValue.startsWith('0x') ? leafValue.slice(2) : leafValue;

        if (cleanValue.length !== 64) {
            console.warn(`Unexpected leaf value length: ${cleanValue.length}, expected 64`);
        }

        const paddedValue = cleanValue.padStart(64, '0');
        const felts: GUint[] = [];

        // The value "0000000000000000000000000000000000000000000000000000000000000bb8"
        // represents 4 felts of 64 bits each (16 hex chars each)
        // Position 0: last 16 chars (rightmost) = "0000000000000bb8" = 0xbb8
        // Position 1: next 16 chars = "0000000000000000" = 0
        // Position 2: next 16 chars = "0000000000000000" = 0
        // Position 3: first 16 chars (leftmost) = "0000000000000000" = 0

        // Parse from right to left (little-endian within the leaf)
        for (let i = 0; i < 4; i++) {
            // Calculate position from the end
            const end = 64 - (i * 16);
            const start = end - 16;
            const feltHex = paddedValue.slice(start, end);
            felts[i] = BigInt('0x' + feltHex);

            if (config.logging.logSignerOps) {
                console.log(`   Position ${i}: ${feltHex} = ${felts[i]}`);
            }
        }

        return felts;
    }

    // Realm-specific state query methods
    async getUserContractTreeRoot(checkpointId: GUint, userId: GUint): Promise<string> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_get_user_contract_tree_root',
            params: [Number(checkpointId), Number(userId)],
            id: this.requestId++,
        };

        const response = await this.realmClient.post<RpcResponse<string>>('', request);
        return response.data.result || '';
    }

    async getUserContractStateTreeRoot(
        checkpointId: GUint,
        userId: GUint,
        contractId: GUint
    ): Promise<string> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_get_user_contract_state_tree_root',
            params: [Number(checkpointId), Number(userId), Number(contractId)],
            id: this.requestId++,
        };

        const response = await this.realmClient.post<RpcResponse<string>>('', request);
        return response.data.result || '';
    }

    async checkUserIdInRealm(userId: GUint): Promise<boolean> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_check_user_id_in_realm',
            params: [Number(userId)],
            id: this.requestId++,
        };

        const response = await this.realmClient.post<RpcResponse<boolean>>('', request);
        return response.data.result || false;
    }

    // ========== COORDINATOR RPC METHODS (Global Operations) ==========

    async getLatestCheckpoint(): Promise<bigint> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_latest_checkpoint',
            params: [],
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse<number>>('', request);

        if (response.data.error) {
            throw new Error(`RPC Error: ${response.data.error.message}`);
        }

        // Default to checkpoint 100 if not available (as shown in Makefile)
        return BigInt(response.data.result || config.checkpoint.defaultId);
    }

    async registerUser(fingerprint: string, publicKeyParam: string): Promise<any> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_register_user',
            params: {
                fingerprint: fingerprint,
                public_key_param: publicKeyParam
            },
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse>('', request);

        if (response.data.error) {
            throw new Error(`RPC Error: ${response.data.error.message}`);
        }

        return response.data.result;
    }

    async deployContract(contractData: any): Promise<any> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_deploy_contract',
            params: contractData,
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse>('', request);

        if (response.data.error) {
            throw new Error(`RPC Error: ${response.data.error.message}`);
        }

        return response.data.result;
    }

    async buildBlock(): Promise<any> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_build_block',
            params: [],
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse>('', request);
        return response.data.result;
    }

    async getLatestL2BlockState(): Promise<any> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_get_latest_l2_block_state',
            params: [],
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse>('', request);
        return response.data.result;
    }

    async getContractLeafData(contractId: GUint): Promise<any> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_get_contract_leaf_data',
            params: [Number(contractId)],
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse>('', request);
        return response.data.result;
    }

    async getUserLeafData(checkpointId: GUint, userId: GUint): Promise<any> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_get_user_leaf_data',
            params: [Number(checkpointId), Number(userId)],
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse>('', request);
        return response.data.result;
    }

    async getContractCodeDefinition(contractId: GUint): Promise<any> {
        const request: RpcRequest = {
            jsonrpc: '2.0',
            method: 'qed_get_contract_code_definition',
            params: [Number(contractId)],
            id: this.requestId++,
        };

        const response = await this.coordinatorClient.post<RpcResponse>('', request);
        return response.data.result;
    }

    // ========== TRANSACTION METHODS ==========

    async sendTransaction(
        contractId: GUint,
        functionName: string,
        args: any[],
        publicKey?: string
    ): Promise<any> {
        try {
            if (!publicKey) {
                throw new Error('Public key required for transaction signing');
            }

            // Look up private key from public key
            let privateKey = this.privateKeys.get(publicKey);
            if (!privateKey) {
                // Try to find in config
                const keyPair = getKeyPairByPublicKey(publicKey);
                if (keyPair) {
                    this.privateKeys.set(keyPair.publicKey, keyPair.privateKey);
                    privateKey = keyPair.privateKey;
                } else {
                    throw new Error(`No private key found for public key: ${publicKey}. Please ensure the key is registered.`);
                }
            }

            if (config.logging.logSignerOps) {
                console.log(`\n🔐 Signing transaction with public key: ${publicKey}`);
            }

            // In the actual QED system, this would:
            // 1. Create the transaction payload
            // 2. Sign it with the private key
            // 3. Submit via psy_user_cli or appropriate RPC method

            // For now, we'll simulate the transaction submission
            const request: RpcRequest = {
                jsonrpc: '2.0',
                method: 'qed_submit_transaction', // Replace with actual method
                params: {
                    contractId: contractId.toString(),
                    methodName: functionName,
                    inputs: args.map(arg => {
                        if (typeof arg === 'bigint') {
                            return arg.toString();
                        }
                        return arg;
                    }),
                    // In production, include signature here
                    signature: this.mockSign(privateKey, contractId, functionName, args),
                    publicKey: publicKey
                },
                id: this.requestId++,
            };

            const response = await this.coordinatorClient.post<RpcResponse>('', request);

            if (response.data.error) {
                throw new Error(`RPC Error: ${response.data.error.message}`);
            }

            console.log(`✅ Transaction submitted successfully`);
            return response.data.result;
        } catch (error) {
            if (axios.isAxiosError(error)) {
                throw new Error(`Network error: ${error.message}`);
            }
            throw error;
        }
    }

    private mockSign(privateKey: string, contractId: GUint, functionName: string, args: any[]): string {
        // Mock signature - in production, use actual cryptographic signing
        const message = `${contractId}-${functionName}-${QedJSON.stringify(args)}`;
        return `sig-${privateKey.substring(0, 8)}-${message.substring(0, 8)}`;
    }

    // ========== UTILITY METHODS ==========

    // Register a private key for a public key (for demo purposes)
    registerKeyPair(publicKey: string, privateKey: string): void {
        this.privateKeys.set(publicKey, privateKey);
        if (config.logging.logSignerOps) {
            console.log(`🔑 Registered key pair for public key: ${publicKey}`);
        }
    }

    // Batch requests support for coordinator
    async batchRequestCoordinator(requests: Omit<RpcRequest, 'jsonrpc' | 'id'>[]): Promise<any[]> {
        return this.batchRequest(this.coordinatorClient, requests);
    }

    // Batch requests support for realm
    async batchRequestRealm(requests: Omit<RpcRequest, 'jsonrpc' | 'id'>[]): Promise<any[]> {
        return this.batchRequest(this.realmClient, requests);
    }

    private async batchRequest(
        client: AxiosInstance,
        requests: Omit<RpcRequest, 'jsonrpc' | 'id'>[]
    ): Promise<any[]> {
        const batchRequests: RpcRequest[] = requests.map((req, index) => ({
            jsonrpc: '2.0',
            ...req,
            id: this.requestId + index,
        }));

        this.requestId += requests.length;

        const response = await client.post<RpcResponse[]>('', batchRequests);

        return response.data.map(res => {
            if (res.error) {
                throw new Error(`Batch request error: ${res.error.message}`);
            }
            return res.result;
        });
    }
}

// Factory function to create providers for different environments
export function createProvider(
    environment: 'local' | 'testnet' | 'mainnet' = 'local',
    realmId?: number
): RpcProvider {
    const rpcUrls = getRpcUrls(environment, realmId);
    return new RpcProvider(rpcUrls.realm, rpcUrls.coordinator);
}

// Create specialized providers for specific use cases
export function createRealmProvider(realmId?: number, environment: 'local' | 'testnet' | 'mainnet' = 'local'): RpcProvider {
    return createProvider(environment, realmId);
}

// Create a provider using custom URLs
export function createCustomProvider(realmUrl: string, coordinatorUrl: string): RpcProvider {
    return new RpcProvider(realmUrl, coordinatorUrl);
}