import { initSync, WasmRpcServer, WasmPsyConfig, WasmPsyConfigBuilder, WasmConstants } from "./psy_prover";
import { wasmBinary } from "./wasm-binary";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    ContractCallData,
    ClaimBatchItem,
    DPNFunctionCircuitDefinition,
    GeneratedTxTraceJson,
    IPsyUserProverProvider,
    SimulatedTxJson,
    QBCDeployContract,
    SignData,
    SignType,
    TraceStepProgressJson,
    TraceProofConcurrentResult,
    TraceProofJobOutputJson,
    TraceProofJobStepIndices,
    TraceProofScheduleJson,
    TxMetadata,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { PsyJSON } from "../utils";
import { PsyNetworkConfig } from "../config";

let isWasmInitialized = false;

// Synchronous WASM initialization function
export function initWasmSync(): void {
    if (isWasmInitialized) {
        return;
    }

    try {
        // Initialize synchronously with pre-compiled binary data
        initSync({ module: wasmBinary });
        isWasmInitialized = true;

        console.log("WASM initialized synchronously from binary data");
    } catch (error) {
        console.error("Failed to initialize WASM:", error);
        throw error;
    }
}

export class PsyWasmWebProverProvider implements IPsyUserProverProvider {
    // Synchronous wasm server handle for backwards compat with wallet callers
    // that read PsyWasmWebProverProvider.wasmServer directly. This field is null
    // during the cold-boot window (before the Plonky2 circuit build resolves)
    // and is populated by ensureWasmServer's promise. New code should use
    // runWasmServerCall / ensureWasmServer instead.
    public static wasmServer: WasmRpcServer | null = null;
    private static wasmServerPromise: Promise<WasmRpcServer> | null = null;
    private static wasmServerConfigJson: string | null = null;
    private static wasmCallQueue: Promise<void> = Promise.resolve();

    constructor(rpcConfigJson: PsyNetworkConfig) {
        const json = PsyJSON.stringify(rpcConfigJson);
        console.log(`WASM init with config: ${json}`);
        void PsyWasmWebProverProvider.ensureWasmServer(json);
    }

    static ensureWasmServer(rpcConfigJson: PsyNetworkConfig | string): Promise<WasmRpcServer> {
        const json = typeof rpcConfigJson === "string" ? rpcConfigJson : PsyJSON.stringify(rpcConfigJson);
        if (!this.wasmServerPromise) {
            const now = new Date().getTime();
            initWasmSync();
            this.wasmServerPromise = Promise.resolve(new WasmRpcServer(json)).then((server) => {
                console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
                this.wasmServer = server;
                return server;
            });
            this.wasmServerConfigJson = json;
        } else if (this.wasmServerConfigJson !== json) {
            console.warn(
                "WASM RPC server is a singleton; ignoring a different config and reusing the existing server."
            );
        }

        return this.wasmServerPromise;
    }

    static runWasmServerCall<T>(callback: (server: WasmRpcServer) => T | Promise<T>): Promise<T> {
        const run = async (): Promise<T> => {
            if (!this.wasmServerPromise) {
                throw new Error("WASM RPC server is not initialized");
            }

            const server = await this.wasmServerPromise;
            return callback(server);
        };

        const result = this.wasmCallQueue.then(run, run);
        this.wasmCallQueue = result.then(
            () => undefined,
            () => undefined,
        );
        return result;
    }

    static async runWasmServerConcurrentCall<T>(callback: (server: WasmRpcServer) => T | Promise<T>): Promise<T> {
        return this.runWasmServerCall(callback);
    }

    async execContractCall(pkHash: string, callData: ContractCallData): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_contract_call_json(pkHash, json)
        );
        console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async execContractCallWithTrace(pkHash: string, callData: ContractCallData): Promise<TxMetadata> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_contract_call_with_trace_json(pkHash, json)
        );
        console.log(`execContractCallWithTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as TxMetadata;
    }

    async claimBatch(pkHash: string, claims: ClaimBatchItem[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_claim_batch_json(pkHash, json)
        );
        console.log(`claimBatch in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async claimBatchWithTrace(pkHash: string, claims: ClaimBatchItem[]): Promise<TxMetadata> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.batch_claim_with_trace_json(pkHash, json)
        );
        console.log(`claimBatchWithTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as TxMetadata;
    }

    async generateBatchClaimTxTrace(pkHash: string, claims: ClaimBatchItem[]): Promise<GeneratedTxTraceJson> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.generate_batch_claim_tx_trace_json(pkHash, json)
        );
        console.log(`generateBatchClaimTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as GeneratedTxTraceJson;
    }

    async batchClaim(pkHash: string, claims: ClaimBatchItem[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.batch_claim_json(pkHash, json)
        );
        console.log(`batchClaim in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async getClaimRewardsCallArgs(_jobInfos: string): Promise<ContractCallArgs[]> {
        throw new Error("Method not implemented.");
    }

    async claimRewards(_pkHash: string, _jobInfos: string): Promise<string> {
        throw new Error("Method not implemented.");
    }

    // Local proving operations
    async startSession(pkHash: PublicKey): Promise<string> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.start_session(pkHash));
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArg);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.prove_contract_call_json(pkHash, json)
        );
        console.log(`proveContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArgs);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.prove_contract_calls_json(pkHash, json)
        );
        console.log(`proveContractCalls in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async signAndSubmit(pkHash: PublicKey, signData?: SignData): Promise<string> {
        const now = new Date().getTime();
        const signDataJson = signData ? PsyJSON.stringify(signData) : null;
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.sign_and_submit(pkHash, signDataJson)
        );
        console.log(`signAndSubmit in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async generateTxTrace(pkHash: PublicKey, callData: ContractCallData): Promise<GeneratedTxTraceJson> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.generate_tx_trace_json(pkHash, json)
        );
        console.log(`generateTxTrace in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as GeneratedTxTraceJson;
    }
    async simulateContractCall(pkHash: PublicKey, callData: ContractCallData): Promise<SimulatedTxJson> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.simulate_contract_call_json(pkHash, json)
        );
        console.log(`simulateContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as SimulatedTxJson;
    }
    // ================================
    // Checkpointed trace proving
    // ================================

    async proveUpsStart(pkHash: PublicKey, envelopeJson: string | GeneratedTxTraceJson): Promise<any> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.prove_ups_start_json(pkHash, envelope)
        ) as Promise<any>;
    }

    async proveTraceStep(
        pkHash: PublicKey,
        envelopeJson: string | GeneratedTxTraceJson,
        stateBlob?: Uint8Array,
        proofs?: Uint8Array[],
    ): Promise<TraceStepProgressJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.prove_trace_step_json(pkHash, envelope, stateBlob, proofs)
        ) as any;
        if (result?.error) throw new Error(String(result.error));
        return result as TraceStepProgressJson;
    }

    async prepareTraceProofSchedule(envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofScheduleJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.prepare_trace_proof_schedule_json(envelope)
        );
    }

    async getTraceProofJobStepIndices(envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofJobStepIndices> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        const raw = await PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.trace_proof_job_step_indices_json(envelope)
        );
        return PsyJSON.parse(raw) as TraceProofJobStepIndices;
    }

    async proveUpsStartJob(pkHash: PublicKey, envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofJobOutputJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.prove_ups_start_job_json(pkHash, envelope)
        );
    }

    async proveCfcJobWithScheduleStep(
        pkHash: PublicKey,
        envelopeJson: string | GeneratedTxTraceJson,
        scheduleJson: TraceProofScheduleJson,
        stepIndex: number,
    ): Promise<TraceProofJobOutputJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.prove_cfc_job_with_schedule_step_json(pkHash, envelope, scheduleJson, stepIndex)
        );
    }

    async proveExternalProofJob(envelopeJson: string | GeneratedTxTraceJson, stepIndex: number): Promise<TraceProofJobOutputJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.prove_external_proof_job_json(envelope, stepIndex)
        );
    }

    async proveZkSignJob(pkHash: PublicKey, envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofJobOutputJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.prove_zksign_job_json(pkHash, envelope)
        );
    }

    async proveEndcapJobFromOutputJsons(
        pkHash: PublicKey,
        envelopeJson: string | GeneratedTxTraceJson,
        scheduleJson: TraceProofScheduleJson,
        outputJsons: TraceProofJobOutputJson[],
    ): Promise<TraceProofJobOutputJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.prove_endcap_job_from_output_jsons_json(pkHash, envelope, scheduleJson, outputJsons)
        );
    }

    async submitEndcapJob(envelopeJson: string | GeneratedTxTraceJson, endcapOutputJson: TraceProofJobOutputJson): Promise<TraceProofJobOutputJson> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerConcurrentCall((server) =>
            server.submit_endcap_job_json(envelope, endcapOutputJson)
        );
    }

    async proveTraceJobsConcurrent(pkHash: PublicKey, envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofConcurrentResult> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        const startedAt = Date.now();
        const logPrefix = "[trace-concurrent]";
        const timedJob = async <T>(name: string, job: () => Promise<T>): Promise<T> => {
            const jobStartedAt = Date.now();
            console.info(`${logPrefix}[${name}] start`);
            try {
                const result = await job();
                console.info(`${logPrefix}[${name}] end`, {
                    durationMs: Date.now() - jobStartedAt,
                });
                return result;
            } catch (error) {
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
            ...indices.cfc_step_indices.map((stepIndex) =>
                timedJob(`cfc:${stepIndex}`, () => this.proveCfcJobWithScheduleStep(pkHash, envelope, scheduleJson, stepIndex))
            ),
            ...indices.external_step_indices.map((stepIndex) =>
                timedJob(`external:${stepIndex}`, () => this.proveExternalProofJob(envelope, stepIndex))
            ),
        ]);
        const endcapOutputJson = await timedJob("endcap", () =>
            this.proveEndcapJobFromOutputJsons(pkHash, envelope, scheduleJson, firstWaveOutputJsons)
        );
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

    async proveEndCapProof(
        pkHash: PublicKey,
        envelopeJson: string | GeneratedTxTraceJson,
        proofTreeMeta: unknown,
        lastStepInfo: unknown,
        allProofBlobs: Uint8Array[],
        signatureProof: Uint8Array,
    ): Promise<any> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.prove_end_cap_proof_json(
                pkHash,
                envelope,
                PsyJSON.stringify(proofTreeMeta),
                PsyJSON.stringify(lastStepInfo),
                allProofBlobs,
                signatureProof,
            )
        ) as Promise<any>;
    }

    async insertExternalProof(
        pkHash: PublicKey,
        envelopeJson: string | GeneratedTxTraceJson,
        proofTreeMeta: unknown,
        lastStepInfo: unknown,
        currentHeader: unknown,
        previousHeader: unknown,
        externalFingerprint: string,
        externalProof: Uint8Array,
    ): Promise<any> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.insert_external_proof_json(
                pkHash,
                envelope,
                PsyJSON.stringify(proofTreeMeta),
                PsyJSON.stringify(lastStepInfo),
                PsyJSON.stringify(currentHeader),
                PsyJSON.stringify(previousHeader),
                externalFingerprint,
                externalProof,
            )
        ) as Promise<any>;
    }

    async submitEndCap(
        envelopeJson: string | GeneratedTxTraceJson,
        endCapProof: Uint8Array,
    ): Promise<string> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.submit_end_cap_json(envelope, endCapProof)
        );
    }

    async signSighash(
        pkHash: PublicKey,
        sighashJson: string,
        envelopeJson?: string | GeneratedTxTraceJson,
        currentHeader?: unknown,
    ): Promise<Uint8Array> {
        const env = envelopeJson ? (typeof envelopeJson === 'string' ? envelopeJson : PsyJSON.stringify(envelopeJson)) : undefined;
        const hdr = currentHeader ? (typeof currentHeader === 'string' ? currentHeader : PsyJSON.stringify(currentHeader)) : undefined;
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.sign_sighash_json(pkHash, sighashJson, env, hdr)
        ) as Promise<Uint8Array>;
    }

    async computeSighashFromEnvelope(
        envelopeJson: string | GeneratedTxTraceJson,
        currentHeader: unknown,
    ): Promise<string> {
        const envelope = typeof envelopeJson === "string" ? envelopeJson : PsyJSON.stringify(envelopeJson);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.compute_sighash_from_envelope_json(envelope, PsyJSON.stringify(currentHeader))
        );
    }



    // User operations
    async registerUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.register_user(privateKey.toString(), signType, fingerprint)
        );
    }

    async addUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.add_user(privateKey.toString(), signType, fingerprint)
        );
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_zk_public_key_json(privateKey.toString())
        );
        return PsyJSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_random_keypair_json()
        );
        return PsyJSON.parse(json);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = PsyJSON.stringify(circuitDefs);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.deploy_contract_json(deployer, json)
        );
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        const json = PsyJSON.stringify(circuitDefs);
        const resultJson = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_deploy_contract_cmd_json(deployer, json)
        );
        return PsyJSON.parse(resultJson);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.ping(message));
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) => server.get_result(id.toString()));
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
