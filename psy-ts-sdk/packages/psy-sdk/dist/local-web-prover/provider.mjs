import { WasmPsyConfigBuilder, WasmConstants, WasmRpcServer, initSync } from './psy_prover.mjs';
import { wasmBinary } from './wasm-binary.mjs';
import '../utils/felt.mjs';
import { PsyJSON } from '../utils/json.mjs';
import '../utils/random.mjs';

let isWasmInitialized = false;
// Synchronous WASM initialization function
function initWasmSync() {
    if (isWasmInitialized) {
        return;
    }
    try {
        // Initialize synchronously with pre-compiled binary data
        initSync({ module: wasmBinary });
        isWasmInitialized = true;
        console.log("WASM initialized synchronously from binary data");
    }
    catch (error) {
        console.error("Failed to initialize WASM:", error);
        throw error;
    }
}
class PsyWasmWebProverProvider {
    constructor(rpcConfigJson) {
        const json = PsyJSON.stringify(rpcConfigJson);
        console.log(`WASM init with config: ${json}`);
        void PsyWasmWebProverProvider.ensureWasmServer(json);
    }
    static ensureWasmServer(rpcConfigJson) {
        const json = typeof rpcConfigJson === "string" ? rpcConfigJson : PsyJSON.stringify(rpcConfigJson);
        if (!this.wasmServer) {
            const now = new Date().getTime();
            initWasmSync();
            this.wasmServer = Promise.resolve(new WasmRpcServer(json)).then((server) => {
                console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
                return server;
            });
            this.wasmServerConfigJson = json;
        }
        else if (this.wasmServerConfigJson !== json) {
            console.warn("WASM RPC server is a singleton; ignoring a different config and reusing the existing server.");
        }
        return this.wasmServer;
    }
    static runWasmServerCall(callback) {
        const run = async () => {
            if (!this.wasmServer) {
                throw new Error("WASM RPC server is not initialized");
            }
            return callback(await this.wasmServer);
        };
        const result = this.wasmCallQueue.then(run, run);
        this.wasmCallQueue = result.then(() => undefined, () => undefined);
        return result;
    }
    async execContractCall(pkHash, callData) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.exec_contract_call_json(pkHash, json));
        console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async execContractCallWithTrace(pkHash, callData) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.exec_contract_call_with_trace_json(pkHash, json));
        console.log(`execContractCallWithTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result);
    }
    async claimBatch(pkHash, claims) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.exec_claim_batch_json(pkHash, json));
        console.log(`claimBatch in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async claimBatchWithTrace(pkHash, claims) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.batch_claim_with_trace_json(pkHash, json));
        console.log(`claimBatchWithTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result);
    }
    async generateBatchClaimTxTrace(pkHash, claims) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.generate_batch_claim_tx_trace_json(pkHash, json));
        console.log(`generateBatchClaimTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result);
    }
    async batchClaim(pkHash, claims) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.batch_claim_json(pkHash, json));
        console.log(`batchClaim in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async getClaimRewardsCallArgs(_jobInfos) {
        throw new Error("Method not implemented.");
    }
    async claimRewards(_pkHash, _jobInfos) {
        throw new Error("Method not implemented.");
    }
    // Local proving operations
    async startSession(pkHash) {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.start_session(pkHash));
    }
    async proveContractCall(pkHash, contractCallArg) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArg);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_contract_call_json(pkHash, json));
        console.log(`proveContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async proveContractCalls(pkHash, contractCallArgs) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArgs);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_contract_calls_json(pkHash, json));
        console.log(`proveContractCalls in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async signAndSubmit(pkHash, signData) {
        const now = new Date().getTime();
        const signDataJson = signData ? PsyJSON.stringify(signData) : null;
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.sign_and_submit(pkHash, signDataJson));
        console.log(`signAndSubmit in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async generateTxTrace(pkHash, callData) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.generate_tx_trace_json(pkHash, json));
        console.log(`generateTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result);
    }
    async simulateContractCall(pkHash, callData) {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.generate_tx_trace_json(pkHash, json));
        console.log(`simulateContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result);
    }
    async proveTxTrace(pkHash, envelopeJson) {
        const now = new Date().getTime();
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_tx_trace_json(pkHash, envelope));
        console.log(`proveTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async proveTxTraceResumable(pkHash, envelopeJson) {
        const now = new Date().getTime();
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_tx_trace_resumable_json(pkHash, envelope));
        console.log(`proveTxTraceResumable in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result);
    }
    // User operations
    async registerUser(privateKey, signType, fingerprint) {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.register_user(privateKey.toString(), signType, fingerprint));
    }
    async addUser(privateKey, signType, fingerprint) {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.add_user(privateKey.toString(), signType, fingerprint));
    }
    async getZKPublicKey(privateKey) {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_zk_public_key_json(privateKey.toString()));
        return PsyJSON.parse(json);
    }
    async getRandomKeypair() {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_random_keypair_json());
        return PsyJSON.parse(json);
    }
    // Contract deployment
    async deployContract(deployer, circuitDefs) {
        const json = PsyJSON.stringify(circuitDefs);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.deploy_contract_json(deployer, json));
    }
    async getDeployContractCmd(deployer, circuitDefs) {
        const json = PsyJSON.stringify(circuitDefs);
        const resultJson = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_deploy_contract_cmd_json(deployer, json));
        return PsyJSON.parse(resultJson);
    }
    // Utility methods
    async ping(message) {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.ping(message));
    }
    async getResult(id) {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_result(id.toString()));
    }
}
PsyWasmWebProverProvider.wasmServer = null;
PsyWasmWebProverProvider.wasmServerConfigJson = null;
PsyWasmWebProverProvider.wasmCallQueue = Promise.resolve();
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
