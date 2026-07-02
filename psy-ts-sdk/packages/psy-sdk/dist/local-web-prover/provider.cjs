'use strict';

var psy_prover = require('./psy_prover.cjs');
var wasmBinary = require('./wasm-binary.cjs');
require('../utils/felt.cjs');
var json = require('../utils/json.cjs');
require('../utils/random.cjs');

let isWasmInitialized = false;
// Synchronous WASM initialization function
function initWasmSync() {
    if (isWasmInitialized) {
        return;
    }
    try {
        // Initialize synchronously with pre-compiled binary data
        psy_prover.initSync({ module: wasmBinary.wasmBinary });
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
        const json$1 = json.PsyJSON.stringify(rpcConfigJson);
        console.log(`WASM init with config: ${json$1}`);
        void PsyWasmWebProverProvider.ensureWasmServer(json$1);
    }
    static ensureWasmServer(rpcConfigJson) {
        const json$1 = typeof rpcConfigJson === "string" ? rpcConfigJson : json.PsyJSON.stringify(rpcConfigJson);
        if (!this.wasmServer) {
            const now = new Date().getTime();
            initWasmSync();
            this.wasmServer = Promise.resolve(new psy_prover.WasmRpcServer(json$1)).then((server) => {
                console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
                return server;
            });
            this.wasmServerConfigJson = json$1;
        }
        else if (this.wasmServerConfigJson !== json$1) {
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
    static async runWasmServerConcurrentCall(callback) {
        return this.runWasmServerCall(callback);
    }
    async execContractCall(pkHash, callData) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.exec_contract_call_json(pkHash, json$1));
        console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async execContractCallWithTrace(pkHash, callData) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.exec_contract_call_with_trace_json(pkHash, json$1));
        console.log(`execContractCallWithTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return json.PsyJSON.parse(result);
    }
    async claimBatch(pkHash, claims) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.exec_claim_batch_json(pkHash, json$1));
        console.log(`claimBatch in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async claimBatchWithTrace(pkHash, claims) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.batch_claim_with_trace_json(pkHash, json$1));
        console.log(`claimBatchWithTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return json.PsyJSON.parse(result);
    }
    async generateBatchClaimTxTrace(pkHash, claims) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.generate_batch_claim_tx_trace_json(pkHash, json$1));
        console.log(`generateBatchClaimTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return json.PsyJSON.parse(result);
    }
    async batchClaim(pkHash, claims) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.batch_claim_json(pkHash, json$1));
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
        const json$1 = json.PsyJSON.stringify(contractCallArg);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_contract_call_json(pkHash, json$1));
        console.log(`proveContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async proveContractCalls(pkHash, contractCallArgs) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(contractCallArgs);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_contract_calls_json(pkHash, json$1));
        console.log(`proveContractCalls in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async signAndSubmit(pkHash, signData) {
        const now = new Date().getTime();
        const signDataJson = signData ? json.PsyJSON.stringify(signData) : null;
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.sign_and_submit(pkHash, signDataJson));
        console.log(`signAndSubmit in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }
    async generateTxTrace(pkHash, callData) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.generate_tx_trace_json(pkHash, json$1));
        console.log(`generateTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return json.PsyJSON.parse(result);
    }
    async simulateContractCall(pkHash, callData) {
        const now = new Date().getTime();
        const json$1 = json.PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.generate_tx_trace_json(pkHash, json$1));
        console.log(`simulateContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return json.PsyJSON.parse(result);
    }
    // ================================
    // Stateless step proving (no WASM state between calls)
    // ================================
    async proveUpsStart(pkHash, envelopeJson) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_ups_start_json(pkHash, envelope));
    }
    async prepareTraceProofSchedule(envelopeJson) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.prepare_trace_proof_schedule_json(envelope));
    }
    async getTraceProofJobStepIndices(envelopeJson) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        const raw = await PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.trace_proof_job_step_indices_json(envelope));
        return json.PsyJSON.parse(raw);
    }
    async proveUpsStartJob(pkHash, envelopeJson) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.prove_ups_start_job_json(pkHash, envelope));
    }
    async proveCfcJobWithScheduleStep(pkHash, envelopeJson, scheduleJson, stepIndex) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.prove_cfc_job_with_schedule_step_json(pkHash, envelope, scheduleJson, stepIndex));
    }
    async proveExternalProofJob(envelopeJson, stepIndex) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.prove_external_proof_job_json(envelope, stepIndex));
    }
    async proveZkSignJob(pkHash, envelopeJson) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.prove_zksign_job_json(pkHash, envelope));
    }
    async proveEndcapJobFromOutputJsons(pkHash, envelopeJson, scheduleJson, outputJsons) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.prove_endcap_job_from_output_jsons_json(pkHash, envelope, scheduleJson, outputJsons));
    }
    async submitEndcapJob(envelopeJson, endcapOutputJson) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) => server.submit_endcap_job_json(envelope, endcapOutputJson));
    }
    async proveTraceJobsConcurrent(pkHash, envelopeJson) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        const startedAt = Date.now();
        const logPrefix = "[trace-concurrent]";
        const timedJob = async (name, job) => {
            const jobStartedAt = Date.now();
            console.info(`${logPrefix}[${name}] start`);
            try {
                const result = await job();
                console.info(`${logPrefix}[${name}] end`, {
                    durationMs: Date.now() - jobStartedAt,
                });
                return result;
            }
            catch (error) {
                console.warn(`${logPrefix}[${name}] failed`, {
                    durationMs: Date.now() - jobStartedAt,
                    error,
                });
                throw error;
            }
        };
        const zkSignPromise = timedJob("zksign", () => this.proveZkSignJob(pkHash, envelope));
        const schedulePromise = timedJob("schedule", () => this.prepareTraceProofSchedule(envelope));
        const indicesPromise = timedJob("indices", () => this.getTraceProofJobStepIndices(envelope));
        const upsStartPromise = timedJob("ups_start", () => this.proveUpsStartJob(pkHash, envelope));
        const [scheduleJson, indices] = await Promise.all([schedulePromise, indicesPromise]);
        const firstWaveOutputJsons = await Promise.all([
            zkSignPromise,
            upsStartPromise,
            ...indices.cfc_step_indices.map((stepIndex) => timedJob(`cfc:${stepIndex}`, () => this.proveCfcJobWithScheduleStep(pkHash, envelope, scheduleJson, stepIndex))),
            ...indices.external_step_indices.map((stepIndex) => timedJob(`external:${stepIndex}`, () => this.proveExternalProofJob(envelope, stepIndex))),
        ]);
        const endcapOutputJson = await timedJob("endcap", () => this.proveEndcapJobFromOutputJsons(pkHash, envelope, scheduleJson, firstWaveOutputJsons));
        const submitOutputJson = await timedJob("submit", () => this.submitEndcapJob(envelope, endcapOutputJson));
        console.info(`${logPrefix}[all] end`, {
            durationMs: Date.now() - startedAt,
        });
        return {
            scheduleJson,
            firstWaveOutputJsons,
            endcapOutputJson,
            submitOutputJson,
        };
    }
    async proveTraceStep(pkHash, envelopeJson, stepIndex, proofTreeMeta, lastStepInfo, currentHeader, previousHeader) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_trace_step_json(pkHash, envelope, stepIndex, json.PsyJSON.stringify(proofTreeMeta), json.PsyJSON.stringify(lastStepInfo), json.PsyJSON.stringify(currentHeader), json.PsyJSON.stringify(previousHeader)));
    }
    async proveEndCapProof(pkHash, envelopeJson, proofTreeMeta, lastStepInfo, allProofBlobs, signatureProof) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.prove_end_cap_proof_json(pkHash, envelope, json.PsyJSON.stringify(proofTreeMeta), json.PsyJSON.stringify(lastStepInfo), allProofBlobs, signatureProof));
    }
    async insertExternalProof(pkHash, envelopeJson, proofTreeMeta, lastStepInfo, currentHeader, previousHeader, externalFingerprint, externalProof) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.insert_external_proof_json(pkHash, envelope, json.PsyJSON.stringify(proofTreeMeta), json.PsyJSON.stringify(lastStepInfo), json.PsyJSON.stringify(currentHeader), json.PsyJSON.stringify(previousHeader), externalFingerprint, externalProof));
    }
    async submitEndCap(envelopeJson, endCapProof) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.submit_end_cap_json(envelope, endCapProof));
    }
    async signSighash(pkHash, sighashJson, envelopeJson, currentHeader) {
        const env = envelopeJson ? (typeof envelopeJson === 'string' ? envelopeJson : json.PsyJSON.stringify(envelopeJson)) : undefined;
        const hdr = currentHeader ? (typeof currentHeader === 'string' ? currentHeader : json.PsyJSON.stringify(currentHeader)) : undefined;
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.sign_sighash_json(pkHash, sighashJson, env, hdr));
    }
    async computeSighashFromEnvelope(envelopeJson, currentHeader) {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : json.PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.compute_sighash_from_envelope_json(envelope, json.PsyJSON.stringify(currentHeader)));
    }
    // User operations
    async registerUser(privateKey, signType, fingerprint) {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.register_user(privateKey.toString(), signType, fingerprint));
    }
    async addUser(privateKey, signType, fingerprint) {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.add_user(privateKey.toString(), signType, fingerprint));
    }
    async getZKPublicKey(privateKey) {
        const json$1 = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_zk_public_key_json(privateKey.toString()));
        return json.PsyJSON.parse(json$1);
    }
    async getRandomKeypair() {
        const json$1 = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_random_keypair_json());
        return json.PsyJSON.parse(json$1);
    }
    // Contract deployment
    async deployContract(deployer, circuitDefs) {
        const json$1 = json.PsyJSON.stringify(circuitDefs);
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.deploy_contract_json(deployer, json$1));
    }
    async getDeployContractCmd(deployer, circuitDefs) {
        const json$1 = json.PsyJSON.stringify(circuitDefs);
        const resultJson = await PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_deploy_contract_cmd_json(deployer, json$1));
        return json.PsyJSON.parse(resultJson);
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
                this._cache = JSON.parse(psy_prover.WasmConstants.getAllConstants());
            }
            catch (err) {
                console.warn("[PsyWasmConstantsProvider] parse error:", err);
                this._cache = {};
            }
        }
        return this._cache;
    }
    static get(key) {
        if (key in psy_prover.WasmConstants) {
            return psy_prover.WasmConstants[key];
        }
        return this.getAll()[key];
    }
    static getRawJson() {
        return psy_prover.WasmConstants.getAllConstants();
    }
    static refresh() {
        this._cache = null;
    }
    static get globalUserTreeHeight() {
        return psy_prover.WasmConstants.global_user_tree_height;
    }
    static get coordinatorUserTreeHeight() {
        return psy_prover.WasmConstants.coordinator_user_tree_height;
    }
    static get realmUserTreeHeight() {
        return psy_prover.WasmConstants.realm_user_tree_height;
    }
    static get groupRealmHeight() {
        return psy_prover.WasmConstants.group_realm_height;
    }
    static get usersPerRealm() {
        return psy_prover.WasmConstants.users_per_realm;
    }
    static get nativeCurrency() {
        return psy_prover.WasmConstants.native_currency;
    }
    static get nativeCurrencyName() {
        return psy_prover.WasmConstants.native_currency_name;
    }
    static get nativeCurrencyDecimal() {
        return psy_prover.WasmConstants.native_currency_decimal;
    }
    static get registerUserFee() {
        return psy_prover.WasmConstants.register_user_fee;
    }
    static get deployContractFee() {
        return psy_prover.WasmConstants.deploy_contract_fee;
    }
    static get gutaFee() {
        return psy_prover.WasmConstants.guta_fee;
    }
    static get currentNetwork() {
        return psy_prover.WasmConstants.current_network;
    }
    static get configPath() {
        return psy_prover.WasmConstants.config_path;
    }
    static get coordinatorRpcUrl() {
        return psy_prover.WasmConstants.coordinator_rpc_url;
    }
    static get realmRpcUrls() {
        return psy_prover.WasmConstants.realm_rpc_urls;
    }
}
PsyWasmConstantsProvider._cache = null;
class PsyWasmConfigBuilderProvider {
    static initBuilder() {
        this.wasmPsyConfigBuilder = new psy_prover.WasmPsyConfigBuilder();
        return this.wasmPsyConfigBuilder;
    }
    static fromJson(json) {
        const builder = new psy_prover.WasmPsyConfigBuilder();
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

exports.PsyWasmConfigBuilderProvider = PsyWasmConfigBuilderProvider;
exports.PsyWasmConstantsProvider = PsyWasmConstantsProvider;
exports.PsyWasmWebProverProvider = PsyWasmWebProverProvider;
exports.initWasmSync = initWasmSync;
