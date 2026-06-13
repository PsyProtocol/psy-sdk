/* tslint:disable */
/* eslint-disable */

export class WasmConstants {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get all constants as a JSON string for easier JS consumption
     */
    static getAllConstants(): string;
    static readonly config_path: string;
    static readonly coordinator_rpc_url: string;
    static readonly coordinator_user_tree_height: number;
    static readonly current_network: string;
    static readonly deploy_contract_fee: bigint;
    static readonly global_user_tree_height: number;
    static readonly group_realm_height: number;
    static readonly guta_fee: bigint;
    static readonly native_currency: string;
    static readonly native_currency_decimal: number;
    static readonly native_currency_name: string;
    static readonly realm_rpc_urls: string[];
    static readonly realm_user_tree_height: number;
    static readonly register_user_fee: bigint;
    static readonly users_per_realm: bigint;
}

export class WasmPsyConfig {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Create using builder pattern (for more complex configurations)
     */
    static builder(): WasmPsyConfigBuilder;
    currentNetworkName(): string;
    getCurrentNetwork(): string;
    getNetworkJson(network_name: string): string;
    listNetworks(): string[];
    constructor(json: string);
    useNetwork(network_name: string): void;
}

/**
 * WASM Builder for flexible configuration in browser/JS environments
 */
export class WasmPsyConfigBuilder {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Build the configuration
     */
    build(): WasmPsyConfig;
    /**
     * Set configuration from JSON string
     */
    json(json: string): WasmPsyConfigBuilder;
    /**
     * Set initial network to use
     */
    network(network: string): WasmPsyConfigBuilder;
    constructor();
}

export class WasmRpcServer {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Inject an external PrivateNoteInclusion proof into the current session tree.
     * Returns JSON: { "leaf_index": u64, "siblings": [[u64;4]] }
     */
    add_external_proof_json(pk_hash: string, note_proof_bincode_b64: string): Promise<string>;
    add_user(private_key_str: string, sign_type: string, sdk_key_fingerprint?: string | null): Promise<string>;
    deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
    exec_claim_batch_json(pk_hash: string, claims_json: string): Promise<string>;
    /**
     * Atomic private_claim flow.
     *
     * This replaces the broken two-step flow (psy_addExternalProof then sendTransaction)
     * where sendTransaction's internal start_session call would reset the session tree,
     * losing the injected external proof.
     *
     * Inputs (all u64 values as decimal strings to avoid JS precision loss):
     *   pk_hash                 - receiver's ZK public key (hex QHashOut)
     *   note_proof_bincode_b64  - base64-encoded PrivateNoteInclusion proof bytes
     *   nullifier_json          - JSON array of 4 decimal strings
     *   owner_json              - JSON array of 4 decimal strings
     *   amount                  - decimal string
     *   user_tree_root_json     - JSON array of 4 decimal strings
     *   checkpoint_id           - decimal string
     *   note_root_slot          - decimal string
     *   contract_id             - decimal string
     *   random0                 - decimal string
     *   random1                 - decimal string
     *
     * Returns the transaction hash string.
     */
    exec_claim_with_external_proof_json(pk_hash: string, note_proof_bincode_b64: string, nullifier_json: string, owner_json: string, amount: string, user_tree_root_json: string, checkpoint_id: string, note_root_slot: string, contract_id: string, random0: string, random1: string): Promise<string>;
    exec_contract_call_json(pk_hash: string, call_data_json: string): Promise<string>;
    /**
     * Atomic shield claim_deposit:
     * Build ShieldDepositClaim proof and submit it atomically.
     *
     * Inputs:
     *   pk_hash                    - receiver's ZK public key (hex QHashOut)
     *   nullifier_json             - JSON array of 4 decimal strings
     *   note_secret_hash_json      - JSON array of 4 decimal strings
     *   token_address_u32x8_json   - JSON array of 8 decimal strings (bytes32 BE words)
     *   l2_token_contract_id_json  - JSON array of 8 decimal strings (bytes32 BE words)
     *   amount_u32x8_json          - JSON array of 8 decimal strings (bytes32 BE words)
     *   source_chain_index         - decimal string
     *   deposit_index              - decimal string
     *   deposit_root_json          - JSON array of 4 decimal strings (QHashOut limbs)
     *   deposit_siblings_json      - JSON array of arrays of 4 decimal strings
     *   random0                    - decimal string
     *   random1                    - decimal string
     *   contract_id                - decimal string
     *
     * Returns the transaction hash string.
     */
    exec_shield_claim_deposit_json(pk_hash: string, nullifier_json: string, note_secret_hash_json: string, token_address_u32x8_json: string, l2_token_contract_id_json: string, amount_u32x8_json: string, source_chain_index: string, deposit_index: string, deposit_root_json: string, deposit_siblings_json: string, random0: string, random1: string, contract_id: string): Promise<string>;
    get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
    get_random_keypair_json(): Promise<string>;
    get_result(id_str: string): Uint8Array;
    get_zk_public_key_json(private_key_str: string): Promise<string>;
    constructor(rpc_config_json: string);
    ping(message: string): string;
    /**
     * Generate a PrivateNoteInclusion ZK proof and return the full NoteProofOutput as JSON.
     *
     * Inputs (all u64 arrays as JSON arrays of decimal strings to avoid JS precision loss):
     *   pk_hash            - sender's ZK public key (hex QHashOut)
     *   owner_json         - receiver's shield address as JSON array of 4 decimal strings
     *   amount             - transfer amount (u64 as decimal string)
     *   note_secret_hash_json - randomness used in commitment, JSON array of 4 decimal strings
     *   nullifier_secret_json - nullifier secret, JSON array of 4 decimal strings
     *   contract_id        - contract ID (u64 as decimal string)
     *   note_root_slot     - note root slot index (u64 as decimal string)
     *   checkpoint_id      - pre-submit checkpoint ID (u64 as decimal string, "0" = latest)
     *
     * Returns JSON matching NoteProofOutput.
     */
    prove_private_note_inclusion_json(pk_hash: string, owner_json: string, amount: string, note_secret_hash_json: string, nullifier_secret_json: string, contract_id: string, note_root_slot: string, checkpoint_id: string): Promise<string>;
    register_sdk_key_circuit(allowed_contract_ids: BigUint64Array, allowed_method_ids: BigUint64Array, expected_tx_count: bigint): Promise<string>;
    register_user(private_key_str: string, sign_type: string, sdk_key_fingerprint?: string | null): Promise<string>;
}

export function init_logging(): void;

export function main(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly __wbg_wasmconstants_free: (a: number, b: number) => void;
    readonly __wbg_wasmpsyconfig_free: (a: number, b: number) => void;
    readonly __wbg_wasmpsyconfigbuilder_free: (a: number, b: number) => void;
    readonly __wbg_wasmrpcserver_free: (a: number, b: number) => void;
    readonly init_logging: () => void;
    readonly main: () => void;
    readonly wasmconstants_config_path: () => [number, number];
    readonly wasmconstants_coordinator_rpc_url: () => [number, number];
    readonly wasmconstants_coordinator_user_tree_height: () => number;
    readonly wasmconstants_current_network: () => [number, number];
    readonly wasmconstants_deploy_contract_fee: () => bigint;
    readonly wasmconstants_getAllConstants: () => [number, number, number, number];
    readonly wasmconstants_global_user_tree_height: () => number;
    readonly wasmconstants_group_realm_height: () => number;
    readonly wasmconstants_guta_fee: () => bigint;
    readonly wasmconstants_native_currency: () => [number, number];
    readonly wasmconstants_native_currency_decimal: () => number;
    readonly wasmconstants_native_currency_name: () => [number, number];
    readonly wasmconstants_realm_rpc_urls: () => [number, number];
    readonly wasmconstants_realm_user_tree_height: () => number;
    readonly wasmconstants_users_per_realm: () => bigint;
    readonly wasmpsyconfig_builder: () => number;
    readonly wasmpsyconfig_currentNetworkName: (a: number) => [number, number];
    readonly wasmpsyconfig_getCurrentNetwork: (a: number) => [number, number, number, number];
    readonly wasmpsyconfig_getNetworkJson: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmpsyconfig_listNetworks: (a: number) => [number, number];
    readonly wasmpsyconfig_new: (a: number, b: number) => [number, number, number];
    readonly wasmpsyconfig_useNetwork: (a: number, b: number, c: number) => [number, number];
    readonly wasmpsyconfigbuilder_build: (a: number) => [number, number, number];
    readonly wasmpsyconfigbuilder_json: (a: number, b: number, c: number) => number;
    readonly wasmpsyconfigbuilder_network: (a: number, b: number, c: number) => number;
    readonly wasmrpcserver_add_external_proof_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_add_user: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wasmrpcserver_deploy_contract_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_exec_claim_batch_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_exec_claim_with_external_proof_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number, w: number) => any;
    readonly wasmrpcserver_exec_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_exec_shield_claim_deposit_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number, w: number, x: number, y: number, z: number, a1: number) => any;
    readonly wasmrpcserver_get_deploy_contract_cmd_json: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmrpcserver_get_random_keypair_json: (a: number) => any;
    readonly wasmrpcserver_get_result: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_get_zk_public_key_json: (a: number, b: number, c: number) => any;
    readonly wasmrpcserver_new: (a: number, b: number) => any;
    readonly wasmrpcserver_ping: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_prove_private_note_inclusion_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number) => any;
    readonly wasmrpcserver_register_sdk_key_circuit: (a: number, b: number, c: number, d: number, e: number, f: bigint) => any;
    readonly wasmrpcserver_register_user: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wasmpsyconfigbuilder_new: () => number;
    readonly wasmconstants_register_user_fee: () => bigint;
    readonly wasm_bindgen__closure__destroy__h1b5505c935284b57: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__h331a7637426ce72b: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hbd96d25d7b63f7a4: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h9bc15f362fb0f120: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hda423c06aeee1c91: (a: number, b: number) => void;
    readonly memory: WebAssembly.Memory;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_thread_destroy: (a?: number, b?: number, c?: number) => void;
    readonly __wbindgen_start: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput, memory?: WebAssembly.Memory, thread_stack_size?: number }} module - Passing `SyncInitInput` directly is deprecated.
 * @param {WebAssembly.Memory} memory - Deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput, memory?: WebAssembly.Memory, thread_stack_size?: number } | SyncInitInput, memory?: WebAssembly.Memory): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput>, memory?: WebAssembly.Memory, thread_stack_size?: number }} module_or_path - Passing `InitInput` directly is deprecated.
 * @param {WebAssembly.Memory} memory - Deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput>, memory?: WebAssembly.Memory, thread_stack_size?: number } | InitInput | Promise<InitInput>, memory?: WebAssembly.Memory): Promise<InitOutput>;
