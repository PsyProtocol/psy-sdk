import { initSync, WasmRpcServer, WasmPsyConfig, WasmPsyConfigBuilder, WasmConstants } from "./psy_prover";
import { wasmBinary } from "./wasm-binary";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    ContractCallData,
    ClaimBatchItem,
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    QBCDeployContract,
    SignData,
    SignType,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { PsyJSON } from "../utils";
import { PsyNetworkConfig } from "../config";

// Synchronous WASM initialization function
export function initWasmSync(): void {
    try {
        // Initialize synchronously with pre-compiled binary data
        initSync({ module: wasmBinary });

        console.log("WASM initialized synchronously from binary data");
    } catch (error) {
        console.error("Failed to initialize WASM:", error);
        throw error;
    }
}

export class PsyWasmWebProverProvider implements IPsyUserProverProvider {
    static wasmServer: WasmRpcServer | Promise<WasmRpcServer> | null = null;

    private static async getWasmServer(): Promise<WasmRpcServer> {
        if (!PsyWasmWebProverProvider.wasmServer) {
            throw new Error("WASM prover is not initialized");
        }
        const server = await PsyWasmWebProverProvider.wasmServer;
        PsyWasmWebProverProvider.wasmServer = server;
        return server;
    }

    constructor(rpcConfigJson: PsyNetworkConfig) {
        const json = PsyJSON.stringify(rpcConfigJson);
        console.log(`WASM init with config: ${json}`);
        if (!PsyWasmWebProverProvider.wasmServer) {
            const now = new Date().getTime();
            initWasmSync();
            PsyWasmWebProverProvider.wasmServer = Promise.resolve(new WasmRpcServer(json)).then((server) => {
                console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
                return server;
            });
        }
    }

    // async execContractCall(pkHash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
    //     const now = new Date().getTime();
    //     const json = PsyJSON.stringify(contractCallArg);
    //     const result = await PsyWasmWebProverProvider.wasmServer.exec_contract_call_json(pkHash, json);
    //     console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
    //     return result;
    // }

    async execContractCall(pkHash: string, callData: ContractCallData): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.exec_contract_call_json(pkHash, json);
        console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async claimBatch(pkHash: string, claims: ClaimBatchItem[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.exec_claim_batch_json(pkHash, json);
        console.log(`claimBatch in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async generateBatchClaimTxTrace(pkHash: string, claims: ClaimBatchItem[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.generate_batch_claim_tx_trace_json(pkHash, json);
        console.log(`generateBatchClaimTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async batchClaim(pkHash: string, claims: ClaimBatchItem[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.batch_claim_json(pkHash, json);
        console.log(`batchClaim in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async getClaimRewardsCallArgs(_jobInfos: string): Promise<ContractCallArgs[]> {
        // const now = new Date().getTime();
        // const json = PsyJSON.stringify(jobInfos);
        // const result = await PsyWasmWebProverProvider.wasmServer.get_claim_rewards_call_args_json(jobInfos);
        // console.log(`claimRewards in ${(new Date().getTime() - now) / 1000} seconds`);
        // const contractCallArgs = PsyJSON.parse(result) as ContractCallArgs[];
        // return contractCallArgs;
        throw new Error("Method not implemented.");
    }

    async claimRewards(_pkHash: string, _jobInfos: string): Promise<string> {
        // const now = new Date().getTime();
        // const json = PsyJSON.stringify(jobInfos);
        // const result = await PsyWasmWebProverProvider.wasmServer.claim_rewards_json(pkHash, jobInfos);
        // console.log(`claimRewards in ${(new Date().getTime() - now) / 1000} seconds`);
        // return result;
        throw new Error("Method not implemented.");
    }

    // Local proving operations
    async startSession(pkHash: PublicKey): Promise<string> {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.start_session(pkHash);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArg);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.prove_contract_call_json(pkHash, json);
        console.log(`proveContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArgs);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.prove_contract_calls_json(pkHash, json);
        console.log(`proveContractCalls in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async signAndSubmit(pkHash: PublicKey, signData?: SignData): Promise<string> {
        const now = new Date().getTime();
        const signDataJson = signData ? PsyJSON.stringify(signData) : null;
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.sign_and_submit(pkHash, signDataJson);
        console.log(`signAndSubmit in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async generateTxTrace(pkHash: PublicKey, callData: ContractCallData): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.generate_tx_trace_json(pkHash, json);
        console.log(`generateTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async simulateContractCall(pkHash: PublicKey, callData: ContractCallData): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.generate_tx_trace_json(pkHash, json);
        console.log(`simulateContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async proveTxTrace(pkHash: PublicKey, envelopeJson: string): Promise<string> {
        const now = new Date().getTime();
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.prove_tx_trace_json(pkHash, envelopeJson);
        console.log(`proveTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }


    // User operations
    async registerUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey> {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.register_user(privateKey.toString(), signType, fingerprint);
    }

    async addUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey> {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.add_user(privateKey.toString(), signType, fingerprint);
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const json = await wasmServer.get_zk_public_key_json(privateKey.toString());
        return PsyJSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const json = await wasmServer.get_random_keypair_json();
        return PsyJSON.parse(json);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = PsyJSON.stringify(circuitDefs);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.deploy_contract_json(deployer, json);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        const json = PsyJSON.stringify(circuitDefs);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const resultJson = await wasmServer.get_deploy_contract_cmd_json(deployer, json);
        return PsyJSON.parse(resultJson);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.ping(message);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.get_result(id.toString());
    }
}

export class PsyWasmConstantsProvider {
    private static _cache: Record<string, any> | null = null;

    static getAll(): Record<string, any> {
        if (!this._cache) {
            try {
                this._cache = JSON.parse(WasmConstants.getAllConstants());
            } catch (err) {
                console.warn("[PsyWasmConstantsProvider] parse error:", err);
                this._cache = {};
            }
        }
        return this._cache!;
    }

    static get<T = any>(key: keyof typeof WasmConstants | string): T {
        if (key in WasmConstants) {
            return (WasmConstants as any)[key] as T;
        }
        return this.getAll()[key as string] as T;
    }

    static getRawJson(): string {
        return WasmConstants.getAllConstants();
    }

    static refresh(): void {
        this._cache = null;
    }

    static get globalUserTreeHeight(): number {
        return WasmConstants.global_user_tree_height;
    }

    static get coordinatorUserTreeHeight(): number {
        return WasmConstants.coordinator_user_tree_height;
    }

    static get realmUserTreeHeight(): number {
        return WasmConstants.realm_user_tree_height;
    }

    static get groupRealmHeight(): number {
        return WasmConstants.group_realm_height;
    }

    static get usersPerRealm(): bigint {
        return WasmConstants.users_per_realm;
    }

    static get nativeCurrency(): string {
        return WasmConstants.native_currency;
    }

    static get nativeCurrencyName(): string {
        return WasmConstants.native_currency_name;
    }

    static get nativeCurrencyDecimal(): number {
        return WasmConstants.native_currency_decimal;
    }

    static get registerUserFee(): bigint {
        return WasmConstants.register_user_fee;
    }

    static get deployContractFee(): bigint {
        return WasmConstants.deploy_contract_fee;
    }

    static get gutaFee(): bigint {
        return WasmConstants.guta_fee;
    }

    static get currentNetwork(): string {
        return WasmConstants.current_network;
    }

    static get configPath(): string {
        return WasmConstants.config_path;
    }

    static get coordinatorRpcUrl(): string {
        return WasmConstants.coordinator_rpc_url;
    }

    static get realmRpcUrls(): string[] {
        return WasmConstants.realm_rpc_urls;
    }
}

export class PsyWasmConfigBuilderProvider {
    static wasmPsyConfigBuilder: WasmPsyConfigBuilder | null = null;
    static wasmPsyConfig: WasmPsyConfig | null = null;

    static initBuilder(): WasmPsyConfigBuilder {
        this.wasmPsyConfigBuilder = new WasmPsyConfigBuilder();
        return this.wasmPsyConfigBuilder;
    }

    static fromJson(json: string): WasmPsyConfigBuilder {
        const builder = new WasmPsyConfigBuilder();
        builder.json(json);
        this.wasmPsyConfigBuilder = builder;
        return builder;
    }

    static setNetwork(network: string): WasmPsyConfigBuilder {
        if (!this.wasmPsyConfigBuilder) this.initBuilder();
        this.wasmPsyConfigBuilder!.network(network);
        return this.wasmPsyConfigBuilder!;
    }

    static build(): WasmPsyConfig {
        if (!this.wasmPsyConfigBuilder)
            throw new Error("WasmPsyConfigBuilder not initialized");
        this.wasmPsyConfig = this.wasmPsyConfigBuilder.build();
        return this.wasmPsyConfig!;
    }

    static useNetwork(network: string): void {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        this.wasmPsyConfig.useNetwork(network);
    }

    static getCurrentNetwork(): string {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.getCurrentNetwork();
    }

    static listNetworks(): string[] {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.listNetworks();
    }

    static getNetworkJson(network: string): string {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.getNetworkJson(network);
    }
}
