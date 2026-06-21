import { WasmPsyConfigBuilder, WasmConstants, WasmRpcServer, initSync } from './psy_prover.mjs';
import { wasmBinary } from './wasm-binary.mjs';
import '../utils/felt.mjs';
import { PsyJSON } from '../utils/json.mjs';
import '../utils/random.mjs';

// Synchronous WASM initialization function
function initWasmSync() {
    try {
        // Initialize synchronously with pre-compiled binary data
        initSync({ module: wasmBinary });
        console.log("WASM initialized synchronously from binary data");
    }
    catch (error) {
        console.error("Failed to initialize WASM:", error);
        throw error;
    }
}
class PsyWasmWebProverProvider {
    static async getWasmServer() {
        if (!PsyWasmWebProverProvider.wasmServer) {
            throw new Error("WASM prover is not initialized");
        }
        const server = await PsyWasmWebProverProvider.wasmServer;
        PsyWasmWebProverProvider.wasmServer = server;
        return server;
    }
    constructor(rpcConfigJson) {
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
    async execContractCall(pkHash, callData) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.exec_contract_call_json(pkHash, json);
        console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async claimBatch(pkHash, claims) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.exec_claim_batch_json(pkHash, json);
        console.log(`claimBatch in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async generateBatchClaimTxTrace(pkHash, claims) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.generate_batch_claim_tx_trace_json(pkHash, json);
        console.log(`generateBatchClaimTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async batchClaim(pkHash, claims) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.batch_claim_json(pkHash, json);
        console.log(`batchClaim in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async getClaimRewardsCallArgs(_jobInfos) {
        // const now = new Date().getTime();
        // const json = PsyJSON.stringify(jobInfos);
        // const result = await PsyWasmWebProverProvider.wasmServer.get_claim_rewards_call_args_json(jobInfos);
        // console.log(`claimRewards in ${(new Date().getTime() - now) / 1000} seconds`);
        // const contractCallArgs = PsyJSON.parse(result) as ContractCallArgs[];
        // return contractCallArgs;
        throw new Error("Method not implemented.");
    }
    async claimRewards(_pkHash, _jobInfos) {
        // const now = new Date().getTime();
        // const json = PsyJSON.stringify(jobInfos);
        // const result = await PsyWasmWebProverProvider.wasmServer.claim_rewards_json(pkHash, jobInfos);
        // console.log(`claimRewards in ${(new Date().getTime() - now) / 1000} seconds`);
        // return result;
        throw new Error("Method not implemented.");
    }
    // Local proving operations
    async startSession(pkHash) {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.start_session(pkHash);
    }
    async proveContractCall(pkHash, contractCallArg) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArg);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.prove_contract_call_json(pkHash, json);
        console.log(`proveContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async proveContractCalls(pkHash, contractCallArgs) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArgs);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.prove_contract_calls_json(pkHash, json);
        console.log(`proveContractCalls in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async signAndSubmit(pkHash, signData) {
        const now = new Date().getTime();
        const signDataJson = signData ? PsyJSON.stringify(signData) : null;
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.sign_and_submit(pkHash, signDataJson);
        console.log(`signAndSubmit in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async generateTxTrace(pkHash, callData) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.generate_tx_trace_json(pkHash, json);
        console.log(`generateTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async simulateContractCall(pkHash, callData) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.generate_tx_trace_json(pkHash, json);
        console.log(`simulateContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async proveTxTrace(pkHash, envelopeJson) {
        const now = new Date().getTime();
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const result = await wasmServer.prove_tx_trace_json(pkHash, envelopeJson);
        console.log(`proveTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    // User operations
    async registerUser(privateKey, signType, fingerprint) {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.register_user(privateKey.toString(), signType, fingerprint);
    }
    async addUser(privateKey, signType, fingerprint) {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.add_user(privateKey.toString(), signType, fingerprint);
    }
    async getZKPublicKey(privateKey) {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const json = await wasmServer.get_zk_public_key_json(privateKey.toString());
        return PsyJSON.parse(json);
    }
    async getRandomKeypair() {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const json = await wasmServer.get_random_keypair_json();
        return PsyJSON.parse(json);
    }
    // Contract deployment
    async deployContract(deployer, circuitDefs) {
        const json = PsyJSON.stringify(circuitDefs);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.deploy_contract_json(deployer, json);
    }
    async getDeployContractCmd(deployer, circuitDefs) {
        const json = PsyJSON.stringify(circuitDefs);
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        const resultJson = await wasmServer.get_deploy_contract_cmd_json(deployer, json);
        return PsyJSON.parse(resultJson);
    }
    // Utility methods
    async ping(message) {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.ping(message);
    }
    async getResult(id) {
        const wasmServer = await PsyWasmWebProverProvider.getWasmServer();
        return wasmServer.get_result(id.toString());
    }
}
PsyWasmWebProverProvider.wasmServer = null;
class PsyWasmConstantsProvider {
    static getAll() {
        if (!this._cache) {
            try {
                this._cache = JSON.parse(WasmConstants.getAllConstants());
            }
            catch (err) {
                console.warn("[PsyWasmConstantsProvider] parse error:", err);
                this._cache = {};
            }
        }
        return this._cache;
    }
    static get(key) {
        if (key in WasmConstants) {
            return WasmConstants[key];
        }
        return this.getAll()[key];
    }
    static getRawJson() {
        return WasmConstants.getAllConstants();
    }
    static refresh() {
        this._cache = null;
    }
    static get globalUserTreeHeight() {
        return WasmConstants.global_user_tree_height;
    }
    static get coordinatorUserTreeHeight() {
        return WasmConstants.coordinator_user_tree_height;
    }
    static get realmUserTreeHeight() {
        return WasmConstants.realm_user_tree_height;
    }
    static get groupRealmHeight() {
        return WasmConstants.group_realm_height;
    }
    static get usersPerRealm() {
        return WasmConstants.users_per_realm;
    }
    static get nativeCurrency() {
        return WasmConstants.native_currency;
    }
    static get nativeCurrencyName() {
        return WasmConstants.native_currency_name;
    }
    static get nativeCurrencyDecimal() {
        return WasmConstants.native_currency_decimal;
    }
    static get registerUserFee() {
        return WasmConstants.register_user_fee;
    }
    static get deployContractFee() {
        return WasmConstants.deploy_contract_fee;
    }
    static get gutaFee() {
        return WasmConstants.guta_fee;
    }
    static get currentNetwork() {
        return WasmConstants.current_network;
    }
    static get configPath() {
        return WasmConstants.config_path;
    }
    static get coordinatorRpcUrl() {
        return WasmConstants.coordinator_rpc_url;
    }
    static get realmRpcUrls() {
        return WasmConstants.realm_rpc_urls;
    }
}
PsyWasmConstantsProvider._cache = null;
class PsyWasmConfigBuilderProvider {
    static initBuilder() {
        this.wasmPsyConfigBuilder = new WasmPsyConfigBuilder();
        return this.wasmPsyConfigBuilder;
    }
    static fromJson(json) {
        const builder = new WasmPsyConfigBuilder();
        builder.json(json);
        this.wasmPsyConfigBuilder = builder;
        return builder;
    }
    static setNetwork(network) {
        if (!this.wasmPsyConfigBuilder)
            this.initBuilder();
        this.wasmPsyConfigBuilder.network(network);
        return this.wasmPsyConfigBuilder;
    }
    static build() {
        if (!this.wasmPsyConfigBuilder)
            throw new Error("WasmPsyConfigBuilder not initialized");
        this.wasmPsyConfig = this.wasmPsyConfigBuilder.build();
        return this.wasmPsyConfig;
    }
    static useNetwork(network) {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        this.wasmPsyConfig.useNetwork(network);
    }
    static getCurrentNetwork() {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.getCurrentNetwork();
    }
    static listNetworks() {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.listNetworks();
    }
    static getNetworkJson(network) {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.getNetworkJson(network);
    }
}
PsyWasmConfigBuilderProvider.wasmPsyConfigBuilder = null;
PsyWasmConfigBuilderProvider.wasmPsyConfig = null;

export { PsyWasmConfigBuilderProvider, PsyWasmConstantsProvider, PsyWasmWebProverProvider, initWasmSync };
