import * as proverModule from "./psy_prover";
import { initSync, WasmRpcServer, WasmPsyConfig, WasmPsyConfigBuilder, WasmConstants } from "./psy_prover";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    ContractCallData,
    ClaimBatchItem,
    DPNFunctionCircuitDefinition,
    GeneratedTxTraceJson,
    IPsyUserProverProvider,
    PreparedRewardClaim,
    PrepareRewardClaimRequest,
    RewardClaimConfirmation,
    RewardClaimPreparation,
    SimulatedTxJson,
    QBCDeployContractV2,
    SignData,
    SignType,
    TraceStepProgressJson,
    TraceProofConcurrentResult,
    TraceProofJobOutputJson,
    TraceProofJobStepIndices,
    TraceProofScheduleJson,
    TxMetadata,
    ViewCallData,
    ViewCallResult,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { PsyJSON } from "../utils";
import { PsyNetworkConfig } from "../config";

let isWasmInitialized = false;
let providedModule: WebAssembly.Module | BufferSource | null = null;

/**
 * Hands the prover module to the SDK: a compiled `WebAssembly.Module` (compile the shipped
 * `psy_prover_bg.wasm` once with `WebAssembly.compileStreaming` and post it to every worker, so they
 * share one copy of the code) or the module's bytes. Call it before the first prover use. Without
 * it the SDK falls back to its embedded base64 copy of the module (54 MB of JavaScript, decoded
 * on first use).
 */
export function setWasmModule(module: WebAssembly.Module | BufferSource): void {
    if (!isWasmInitialized) {
        providedModule = module;
    }
}

/**
 * Initialises the prover synchronously from `module`, or from the module given to
 * `setWasmModule`. Throws if neither is available; `PsyWasmWebProverProvider.ensureWasmServer`
 * falls back to the embedded copy instead.
 */
export function initWasmSync(module?: WebAssembly.Module | BufferSource): void {
    if (isWasmInitialized) {
        return;
    }
    const source = module ?? providedModule;
    if (!source) {
        throw new Error("Prover WASM module not provided: call setWasmModule() or use PsyWasmWebProverProvider.ensureWasmServer()");
    }
    try {
        wasmMemory = (initSync({ module: source }) as unknown as { memory: WebAssembly.Memory }).memory;
        isWasmInitialized = true;
        providedModule = null;
        console.log("WASM initialized");
    } catch (error) {
        console.error("Failed to initialize WASM:", error);
        throw error;
    }
}

let threadCount = 0;
let wasmMemory: WebAssembly.Memory | null = null;

/**
 * Proving threads to start after init (0 = none, the default). Takes effect only with a prover
 * built with the `threads` feature, in a cross-origin isolated context (SharedArrayBuffer);
 * otherwise proving stays on the calling thread. Call it before `ensureWasmServer`.
 */
export function setThreadCount(threads: number): void {
    threadCount = Math.max(0, Math.floor(threads));
}

type ThreadGlobals = { __psyProverThreadWorkerFactory?: () => Worker };

/**
 * How to start one proving thread: return a new module Worker whose script calls
 * `startProverThreadWorker()` (it must bundle the same SDK build). Hosts bundle that script
 * themselves, e.g. `() => new Worker(new URL("./prover-thread.ts", import.meta.url), { type: "module" })`.
 * Required for `setThreadCount`.
 */
export function setThreadWorkerFactory(factory: () => Worker): void {
    (globalThis as ThreadGlobals).__psyProverThreadWorkerFactory = factory;
}

// The thread-worker side lives in ./thread, which imports only the wasm-bindgen glue, so a host's
// thread-worker bundle stays small.
export { startProverThreadWorker } from "./thread";

async function startThreadPool(): Promise<void> {
    const initThreadPool = (proverModule as { initThreadPool?: (n: number) => Promise<unknown> }).initThreadPool;
    if (threadCount < 2 || !initThreadPool) {
        return;
    }
    if (!(globalThis as { crossOriginIsolated?: boolean }).crossOriginIsolated) {
        console.warn("Prover threads need cross-origin isolation; proving on one thread");
        return;
    }
    if (typeof (globalThis as ThreadGlobals).__psyProverThreadWorkerFactory !== "function") {
        console.warn("Prover threads need setThreadWorkerFactory(); proving on one thread");
        return;
    }
    try {
        await initThreadPool(threadCount);
    } catch (error) {
        // Never fail initialisation over the pool: the prover is already initialised, and rayon
        // without a global pool runs on the calling thread.
        console.warn("Prover thread pool failed to start; proving on one thread:", error);
        return;
    }
    console.log(`Prover thread pool started with ${threadCount} threads`);
}

// LAB ONLY (local-proving lab): Merkle commitments on WebGPU. A second worker owns the GPU device
// and reads the prover's shared memory; the prover blocks in Atomics.wait for each tree (see the
// lab's harness/static/gpu-worker.js for the worker side and the message protocol).
type GpuGlobals = {
    __psyProverGpuWorkerFactory?: () => Worker;
    labGpuTree?: (dataPtr: number, n: number, leafLen: number, levels: number, outPtr: number) => boolean;
    labGpuCommitStart?: (coeffPtr: number, polys: number, polyLen: number, rateBits: number, levels: number, outPtr: number) => boolean;
    labGpuCommitWait?: () => boolean;
    labGpuPow?: (statePtr: number, pos: number, bits: number) => number;
    __psyProverGpuStatus?: unknown;
};
type LabExports = {
    lab_poseidon_constants?: () => string;
    lab_poseidon_hash_leaves?: (data: Uint32Array, leafLen: number) => Uint32Array;
    lab_ntt_tables?: (logN: number, rateBits: number) => Uint32Array;
    lab_gpu_merkle_mode?: (mode: number) => void;
    lab_gpu_tree_counts?: () => Uint32Array;
};

/** LAB ONLY. Timeline lines (RPCs, local proofs, phases) recorded since the last call, or null. */
export function timelineTake(): { now_ms: number; lines: Array<Record<string, unknown>> } | null {
    const take = (proverModule as unknown as { lab_timeline_take?: () => string }).lab_timeline_take;
    if (!isWasmInitialized || !take) {
        return null;
    }
    // Times and sizes only: safe to parse as JavaScript numbers.
    return JSON.parse(take());
}

let oneShot = false;

/** LAB ONLY. Prove a transaction with a single prove-proxy request when the proxy supports it. Call before first use. */
export function setOneShotProxy(enabled: boolean): void {
    oneShot = enabled;
}

let localProve = false;

/** LAB ONLY. Prove whole transactions in this prover instead of sending them to the prove proxy. Call before first use. */
export function setLocalProve(enabled: boolean): void {
    localProve = enabled;
}

/** LAB ONLY. How to start the GPU worker (a module Worker running the lab's gpu-worker.js). */
export function setGpuWorkerFactory(factory: () => Worker): void {
    (globalThis as GpuGlobals).__psyProverGpuWorkerFactory = factory;
}

/** LAB ONLY. `[accepted, rejected]` GPU trees so far, or null without the lab exports. */
export function gpuTreeCounts(): number[] | null {
    const counts = (proverModule as unknown as LabExports).lab_gpu_tree_counts;
    return counts ? Array.from(counts()) : null;
}

async function startGpu(): Promise<void> {
    const globals = globalThis as GpuGlobals;
    const lab = proverModule as unknown as LabExports;
    const factory = globals.__psyProverGpuWorkerFactory;
    const status = (value: unknown) => {
        globals.__psyProverGpuStatus = value;
        console.log("[lab-gpu]", JSON.stringify(value));
    };
    if (typeof factory !== "function") {
        return;
    }
    if (!wasmMemory || !(wasmMemory.buffer instanceof SharedArrayBuffer) || !lab.lab_gpu_merkle_mode || !lab.lab_poseidon_constants || !lab.lab_poseidon_hash_leaves || !lab.lab_ntt_tables) {
        status({ useGpu: false, reason: "prover without shared memory or lab exports" });
        return;
    }
    try {
        const ntt = lab.lab_ntt_tables;
        const ctrl = new Int32Array(new SharedArrayBuffer(16));
        const worker = factory();
        // The same leaves on both sides decide whether the GPU is worth using on this device.
        const calN = 4096;
        const calLeaf = 135;
        const calData = new Uint32Array(calN * calLeaf * 2);
        for (let i = 0, x = 0x9e3779b9; i < calData.length; i += 1) {
            x ^= x << 13;
            x ^= x >>> 17;
            x ^= x << 5;
            calData[i] = x >>> 0;
        }
        let cpuMs = Infinity;
        for (let i = 0; i < 2; i += 1) {
            const t = performance.now();
            lab.lab_poseidon_hash_leaves(calData, calLeaf);
            cpuMs = Math.min(cpuMs, performance.now() - t);
        }
        const ready = new Promise<{ ok: boolean; error?: string; adapter?: unknown; calibrationMs?: number }>((resolve) => {
            worker.onmessage = (event) => resolve(event.data);
            worker.onerror = (event) => resolve({ ok: false, error: `gpu worker failed to load: ${event.message || event.type}` });
            setTimeout(() => resolve({ ok: false, error: "no answer in 20 s" }), 20000);
        });
        worker.postMessage({
            memory: wasmMemory,
            ctrl: ctrl.buffer,
            constants: JSON.parse(lab.lab_poseidon_constants()),
            calibrate: { data: calData, n: calN, leafLen: calLeaf },
        });
        const info = await ready;
        worker.onmessage = null;
        if (!info.ok || typeof info.calibrationMs !== "number") {
            status({ useGpu: false, reason: info.error ?? "no calibration" });
            worker.terminate();
            return;
        }
        if (info.calibrationMs >= cpuMs) {
            status({ useGpu: false, reason: "GPU slower than CPU", adapter: info.adapter, gpuMs: info.calibrationMs, cpuMs });
            worker.terminate();
            return;
        }
        globals.labGpuTree = (dataPtr, n, leafLen, levels, outPtr) => {
            Atomics.store(ctrl, 0, 0);
            worker.postMessage({ dataPtr, n, leafLen, levels, outPtr });
            Atomics.wait(ctrl, 0, 0, 60000);
            return Atomics.load(ctrl, 0) === 1;
        };
        const tableKeys = new Set<string>();
        globals.labGpuCommitStart = (coeffPtr, polys, polyLen, rateBits, levels, outPtr) => {
            const key = `${polyLen}:${rateBits}`;
            const tables = tableKeys.has(key) ? null : ntt(Math.log2(polyLen), rateBits);
            tableKeys.add(key);
            Atomics.store(ctrl, 0, 0);
            worker.postMessage({ commit: true, coeffPtr, polys, polyLen, rateBits, levels, outPtr, tables });
            return true;
        };
        globals.labGpuCommitWait = () => {
            Atomics.wait(ctrl, 0, 0, 60000);
            return Atomics.load(ctrl, 0) === 1;
        };
        // The FRI proof-of-work search: returns the witness, or -1 (the prover then searches itself).
        globals.labGpuPow = (statePtr, pos, bits) => {
            Atomics.store(ctrl, 0, 0);
            worker.postMessage({ pow: true, statePtr, pos, bits });
            Atomics.wait(ctrl, 0, 0, 60000);
            return Atomics.load(ctrl, 0) === 1 ? (Atomics.load(ctrl, 1) >>> 0) + (Atomics.load(ctrl, 2) >>> 0) * 2 ** 32 : -1;
        };
        lab.lab_gpu_merkle_mode(3);
        status({ useGpu: true, adapter: info.adapter, gpuMs: info.calibrationMs, cpuMs });
    } catch (error) {
        status({ useGpu: false, reason: String(error) });
    }
}

/** Initialises from the provided module, else from the embedded base64 copy (imported only now). */
async function initWasm(): Promise<void> {
    if (isWasmInitialized) {
        return;
    }
    if (!providedModule) {
        const { wasmBinary } = await import("./wasm-binary");
        initWasmSync(wasmBinary);
    } else {
        initWasmSync();
    }
    (proverModule as unknown as { lab_timeline_enable?: (enabled: boolean) => void }).lab_timeline_enable?.(true);
    await startThreadPool();
    await startGpu();
    if (oneShot) {
        (proverModule as unknown as { lab_set_one_shot?: (enabled: boolean) => void }).lab_set_one_shot?.(true);
    }
    if (localProve) {
        const set = (proverModule as unknown as { lab_set_local_prove?: (enabled: boolean) => void }).lab_set_local_prove;
        if (set) {
            set(true);
            console.log("[lab-local-prove] wallet sessions prove transactions locally");
        } else {
            console.warn("[lab-local-prove] this prover build has no lab_set_local_prove");
        }
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
            this.wasmServerPromise = initWasm().then(() => new WasmRpcServer(json)).then((server) => {
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

    async proveTxTrace(pkHash: string, trace: string | GeneratedTxTraceJson): Promise<string> {
        const envelope = typeof trace === "string" ? trace : PsyJSON.stringify(trace);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.prove_tx_trace_json(pkHash, envelope)
        );
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

    async prepareRewardClaim(pkHash: string, request: PrepareRewardClaimRequest): Promise<RewardClaimPreparation> {
        const requestJson = PsyJSON.stringify(request);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.prepare_reward_claim_json(pkHash, requestJson)
        );
        return PsyJSON.parse(result) as RewardClaimPreparation;
    }

    async confirmRewardClaim(l1RpcUrl: string, prepared: PreparedRewardClaim, txHash: string): Promise<RewardClaimConfirmation> {
        const preparedJson = PsyJSON.stringify(prepared);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.confirm_reward_claim_json(l1RpcUrl, preparedJson, txHash)
        );
        return PsyJSON.parse(result) as RewardClaimConfirmation;
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
    async callView(pkHash: PublicKey, callData: ViewCallData): Promise<ViewCallResult> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.call_view_json(pkHash, json)
        );
        console.log(`callView in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as ViewCallResult;
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

    // External EIP-191 personal_sign signer (browser / MetaMask flow).
    // Hex inputs accept an optional `0x` prefix (enforced by the WASM adapter);
    // the SDK forwards them verbatim and performs no cryptography itself.
    async ethPersonalRegistrationChallenge(selectedEvmAddressHex: string): Promise<string> {
        return PsyWasmWebProverProvider.runWasmServerCall(() =>
            WasmRpcServer.eth_personal_registration_challenge(selectedEvmAddressHex)
        );
    }

    async registerExternalEthPersonalUser(selectedEvmAddressHex: string, recoveryMessageHex32: string, signatureHex65: string): Promise<PublicKey> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.register_external_eth_personal_user(selectedEvmAddressHex, recoveryMessageHex32, signatureHex65)
        );
    }

    async injectEthPersonalSignature(expectedPkHash: PublicKey, selectedEvmAddressHex: string, messageHex32: string, signatureHex65: string): Promise<PublicKey> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.inject_eth_personal_signature(expectedPkHash, selectedEvmAddressHex, messageHex32, signatureHex65)
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
    async deployContract(deployer: bigint, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = PsyJSON.stringify(circuitDefs);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.deploy_contract_json(deployer, json)
        );
    }

    async getDeployContractCmd(
        deployer: bigint,
        circuitDefs: DPNFunctionCircuitDefinition[],
        abi: unknown
    ): Promise<QBCDeployContractV2> {
        if (abi == null) {
            throw new Error("Contract ABI is required to build a layout-aware deploy command");
        }
        const json = PsyJSON.stringify(circuitDefs);
        const abiJson = PsyJSON.stringify(abi);
        const resultJson = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_layout_aware_deploy_contract_cmd_json(deployer, json, abiJson)
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
