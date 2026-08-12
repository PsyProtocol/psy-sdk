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
    add_external_proof_json(pk_hash: string, note_proof_bincode_b64: string, note_proof_fingerprint_json?: string | null, note_verifier_data_json?: string | null): Promise<string>;
    add_user(private_key_str: string, sign_type: string, fingerprint?: string | null): Promise<string>;
    batch_claim_json(pk_hash: string, items_json: string): Promise<string>;
    batch_claim_with_trace_json(pk_hash: string, items_json: string): Promise<string>;
    call_view_json(pk_hash: string, call_data_json: string): Promise<string>;
    /**
     * Compute sighash from an envelope + current header JSON.
     * Extracts nonce, user_id, and network_magic from the trace itself,
     * so JS doesn't need to parse the bincode payload.
     */
    compute_sighash_from_envelope_json(envelope_json: string, current_header_json: string): string;
    deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
    /**
     * Returns the exact 32-byte, network-bound challenge that the selected
     * account must sign before external EIP-191 registration.
     */
    static eth_personal_registration_challenge(selected_evm_address_hex: string): Promise<string>;
    exec_claim_batch_json(pk_hash: string, claims_json: string): Promise<string>;
    /**
     * Atomic private_claim: start_session → add_external_proof → prove → sign_and_submit.
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
    exec_claim_with_external_proof_json(pk_hash: string, note_proof_bincode_b64: string, nullifier_json: string, owner_json: string, amount: string, user_tree_root_json: string, checkpoint_id: string, note_root_slot: string, contract_id: string, random0: string, random1: string, note_proof_fingerprint_json?: string | null, note_verifier_data_json?: string | null): Promise<string>;
    exec_contract_call_json(pk_hash: string, call_data_json: string): Promise<string>;
    exec_contract_call_with_trace_json(pk_hash: string, call_data_json: string): Promise<string>;
    /**
     * Inputs:
     *   pk_hash                    - receiver's ZK public key (hex QHashOut)
     *   token_address_u32x8_json   - JSON array of 8 decimal strings (bytes32 BE words)
     *   l2_token_contract_id_json  - JSON array of 8 decimal strings (bytes32 BE words)
     *   amount_u32x8_json          - JSON array of 8 decimal strings (bytes32 BE words)
     *   source_chain_index         - decimal string
     *   deposit_index              - decimal string
     *   deposit_root_json          - JSON array of 4 decimal strings (QHashOut limbs)
     *   nullifier_hash_json        - JSON array of 4 decimal strings (QHashOut limbs)
     *   note_commitment_json       - JSON array of 4 decimal strings (QHashOut limbs)
     *   deposit_proof_bincode_b64  - base64-encoded bincode ProofWithPublicInputs
     *   random0                    - decimal string (receiver's r0, locally derived)
     *   random1                    - decimal string (receiver's r1, locally derived)
     *   contract_id                - decimal string
     *   deposit_proof_fingerprint_json - optional JSON array of 4 decimal strings (circuit fingerprint)
     *
     * Returns the transaction hash string.
     */
    exec_shield_claim_deposit_json(pk_hash: string, token_address_u32x8_json: string, l2_token_contract_id_json: string, amount_u32x8_json: string, source_chain_index: string, deposit_index: string, deposit_root_json: string, nullifier_hash_json: string, note_commitment_json: string, deposit_proof_bincode_b64: string, random0: string, random1: string, contract_id: string, deposit_proof_fingerprint_json?: string | null): Promise<string>;
    generate_batch_claim_tx_trace_json(pk_hash: string, items_json: string): Promise<string>;
    generate_tx_trace_json(pk_hash: string, call_data_json: string): Promise<string>;
    get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
    get_random_keypair_json(): Promise<string>;
    get_result(id_str: string): Uint8Array;
    get_zk_public_key_json(private_key_str: string): Promise<string>;
    /**
     * Inject the MetaMask `r || s || v` signature for the exact raw 32-byte
     * session sighash. The wallet session recovers and authenticates the
     * signer against both the selected EVM address and registered key hash.
     */
    inject_eth_personal_signature(expected_pk_hash: string, selected_evm_address_hex: string, sighash_hex: string, signature_hex: string): Promise<string>;
    /**
     * Stateless external proof insertion: inject a private_note_inclusion or
     * shield_deposit_claim proof into the proof tree. No baton/header changes.
     * Returns the updated `proof_tree_meta` with the new leaf's metadata
     * appended to `leaf_records`.
     */
    insert_external_proof_json(pk_hash: string, envelope_json: string, proof_tree_meta_json: string, last_step_info_json: string, current_header_json: string, previous_header_json: string, external_fingerprint: string, external_proof: Uint8Array): Promise<any>;
    constructor(rpc_config_json: string);
    ping(message: string): string;
    prepare_trace_proof_schedule_json(envelope_json: string): Promise<string>;
    prove_cfc_job_with_schedule_step_json(pk_hash: string, envelope_json: string, schedule_json: string, step_index: number): Promise<string>;
    prove_contract_call_json(pk_hash: string, contract_call_json: string): Promise<string>;
    prove_contract_calls_json(pk_hash: string, contract_calls_json: string): Promise<string>;
    /**
     * Atomic shield claim_deposit:
     * build ShieldDepositClaim proof -> start_session -> add_external_proof -> prove -> sign_and_submit.
     *
     * Generate a sender-side DepositInclusion proof packet without
     * submitting any claim transaction.
     *
     * Inputs:
     *   shield_address_json         - JSON array of 4 decimal strings
     *   nullifier_json             - JSON array of 4 decimal strings
     *   note_secret_json           - JSON array of 4 decimal strings
     *   token_address_u32x8_json   - JSON array of 8 decimal strings (bytes32 BE words)
     *   l2_token_contract_id_json  - JSON array of 8 decimal strings (bytes32 BE words)
     *   amount_u32x8_json          - JSON array of 8 decimal strings (bytes32 BE words)
     *   source_chain_index         - decimal string
     *   deposit_index              - decimal string
     *   deposit_root_json          - JSON array of 4 decimal strings (QHashOut limbs)
     *   deposit_siblings_json      - JSON array of arrays of 4 decimal strings
     *
     * Returns JSON containing:
     *   deposit_proof_bincode_b64
     *   deposit_proof_fingerprint
     *   shield_address
     *   amount_u32x8
     *   token_address_u32x8
     *   l2_token_contract_id
     *   source_chain_index
     *   deposit_index
     *   deposit_root
     *   nullifier_hash
     *   note_commitment
     */
    prove_deposit_inclusion_json(shield_address_json: string, nullifier_json: string, note_secret_json: string, token_address_u32x8_json: string, l2_token_contract_id_json: string, amount_u32x8_json: string, source_chain_index: string, deposit_index: string, deposit_root_json: string, deposit_siblings_json: string): Promise<string>;
    /**
     * Stateless end-cap prove: reconstructs all leaf_proofs from JS-provided records,
     * adds ZkSign leaf, runs finalize_tree. Takes external signature proof.
     * `all_proof_blobs` are bincode-serialized `ProofWithPublicInputs` for
     * each leaf in insertion order (from trace cfc_proof/ups_proof).
     * `proof_tree_meta` must contain `leaf_records` with `insertion_proof`.
     */
    prove_end_cap_proof_json(pk_hash: string, envelope_json: string, proof_tree_meta_json: string, last_step_info_json: string, all_proof_blobs: Uint8Array[], signature_proof: Uint8Array): Promise<any>;
    prove_endcap_job_from_output_jsons_json(pk_hash: string, envelope_json: string, schedule_json: string, output_jsons: string[]): Promise<string>;
    prove_external_proof_job_json(envelope_json: string, step_index: number): Promise<string>;
    /**
     * Generate a PrivateNoteInclusion ZK proof and return the full NoteProofOutput as JSON.
     *
     * Inputs (u64 arrays use JSON arrays of decimal strings to avoid JS precision loss):
     *   pk_hash            - sender's ZK public key (hex QHashOut)
     *   owner_json         - receiver's shield address as JSON array of 4 decimal strings
     *   amount             - transfer amount (u64 as decimal string)
     *   note_secret_json   - randomness used in commitment, JSON array of 4 decimal strings
     *   nullifier_secret_json - nullifier secret, JSON array of 4 decimal strings
     *   contract_id        - contract ID (u64 as decimal string)
     *   note_root_slot     - note root slot index (u64 as decimal string)
     *   checkpoint_id      - immutable pre-submit checkpoint ID (u64 as decimal string)
     *   end_user_leaf_hash - submitted transaction's end-user leaf hash (hex QHashOut)
     *
     * Returns JSON matching NoteProofOutput.
     */
    prove_private_note_inclusion_json(pk_hash: string, owner_json: string, amount: string, note_secret_json: string, nullifier_secret_json: string, contract_id: string, note_root_slot: string, checkpoint_id: string, end_user_leaf_hash: string): Promise<string>;
    prove_trace_step_json(pk_hash: string, envelope_json: string, state_blob?: Uint8Array | null, proofs?: Uint8Array[] | null): Promise<any>;
    prove_ups_start_job_json(pk_hash: string, envelope_json: string): Promise<string>;
    prove_ups_start_json(pk_hash: string, envelope_json: string): Promise<any>;
    prove_zksign_job_json(pk_hash: string, envelope_json: string): Promise<string>;
    /**
     * Register an externally held EIP-191 secp256k1 key. The wallet session
     * recovers the signer from the selected EVM address, recovery message, and
     * MetaMask `r || s || v` signature; callers cannot supply a public key.
     */
    register_external_eth_personal_user(selected_evm_address_hex: string, recovery_message_hex: string, signature_hex: string): Promise<string>;
    register_sd_key_circuit(allowed_contract_ids: BigUint64Array, allowed_method_ids: BigUint64Array, expected_tx_count: bigint): Promise<string>;
    register_user(private_key_str: string, sign_type: string, fingerprint?: string | null): Promise<string>;
    sign_and_submit(pk_hash: string, sign_data?: string | null): Promise<string>;
    /**
     * Sign a sighash with the wallet's private key and return the signature
     * proof as bincode bytes (Uint8Array). Used by the step proving path:
     * JS calls `compute_sighash_from_envelope_json` → `sign_sighash_json` →
     * passes the result to `prove_end_cap_proof_json`.
     *
     * NOTE: This still uses the wallet's in-WASM private key. Full signer
     * externalisation (Phase 2) would move this to JS.
     */
    sign_sighash_json(pk_hash: string, sighash_json: string, envelope_json?: string | null, current_header_json?: string | null): Promise<Uint8Array>;
    simulate_contract_call_json(pk_hash: string, call_data_json: string): Promise<string>;
    start_session(pk_hash: string): Promise<string>;
    /**
     * Submit a pre-proven end-cap proof (RPC only, no proving).
     */
    submit_end_cap_json(envelope_json: string, end_cap_proof: Uint8Array): Promise<string>;
    submit_endcap_job_json(envelope_json: string, endcap_output_json: string): Promise<string>;
    trace_proof_job_step_indices_json(envelope_json: string): string;
}

export function init_logging(): void;

export function main(): void;

export type SyncInitInput = BufferSource | WebAssembly.Module;

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
    readonly wasmrpcserver_add_external_proof_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmrpcserver_add_user: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wasmrpcserver_batch_claim_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_batch_claim_with_trace_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_call_view_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_compute_sighash_from_envelope_json: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmrpcserver_deploy_contract_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_eth_personal_registration_challenge: (a: number, b: number) => any;
    readonly wasmrpcserver_exec_claim_batch_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_exec_claim_with_external_proof_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number, w: number, x: number, y: number, z: number, a1: number) => any;
    readonly wasmrpcserver_exec_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_exec_contract_call_with_trace_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_exec_shield_claim_deposit_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number, w: number, x: number, y: number, z: number, a1: number, b1: number, c1: number) => any;
    readonly wasmrpcserver_generate_batch_claim_tx_trace_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_generate_tx_trace_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_get_deploy_contract_cmd_json: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmrpcserver_get_random_keypair_json: (a: number) => any;
    readonly wasmrpcserver_get_result: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_get_zk_public_key_json: (a: number, b: number, c: number) => any;
    readonly wasmrpcserver_inject_eth_personal_signature: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmrpcserver_insert_external_proof_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number) => any;
    readonly wasmrpcserver_new: (a: number, b: number) => any;
    readonly wasmrpcserver_ping: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_prepare_trace_proof_schedule_json: (a: number, b: number, c: number) => any;
    readonly wasmrpcserver_prove_cfc_job_with_schedule_step_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => any;
    readonly wasmrpcserver_prove_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_prove_contract_calls_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_prove_deposit_inclusion_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number) => any;
    readonly wasmrpcserver_prove_end_cap_proof_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number) => any;
    readonly wasmrpcserver_prove_endcap_job_from_output_jsons_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmrpcserver_prove_external_proof_job_json: (a: number, b: number, c: number, d: number) => any;
    readonly wasmrpcserver_prove_private_note_inclusion_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number) => any;
    readonly wasmrpcserver_prove_trace_step_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmrpcserver_prove_ups_start_job_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_prove_ups_start_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_prove_zksign_job_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_register_external_eth_personal_user: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wasmrpcserver_register_sd_key_circuit: (a: number, b: number, c: number, d: number, e: number, f: bigint) => any;
    readonly wasmrpcserver_register_user: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wasmrpcserver_sign_and_submit: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_sign_sighash_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmrpcserver_simulate_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_start_session: (a: number, b: number, c: number) => any;
    readonly wasmrpcserver_submit_end_cap_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_submit_endcap_job_json: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrpcserver_trace_proof_job_step_indices_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmpsyconfigbuilder_new: () => number;
    readonly wasmconstants_register_user_fee: () => bigint;
    readonly wasm_bindgen__convert__closures_____invoke__h447e0f573cfb1039: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h205d9aeaebc44d62: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__habcecd6f13b274cc: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hce85efe9a3522159: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h780b2bd6838c983d: (a: number, b: number) => void;
    readonly memory: WebAssembly.Memory;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_thread_destroy: (a?: number, b?: number, c?: number) => void;
    readonly __wbindgen_start: (a: number) => void;
}

export interface InitSyncOptions {
    module?: SyncInitInput;
    memory?: WebAssembly.Memory;
    thread_stack_size?: number;
}

/**
 * Initialize the WebAssembly module synchronously.
 *
 * For the main thread, this is called automatically on import.
 * Worker threads should call this explicitly with shared module and memory:
 *
 * ```js
 * initSync({ module: __wbg_wasm_module, memory: __wbg_memory });
 * ```
 *
 * @param opts - Initialization options
 * @returns The exports object
 */
export function initSync(opts?: InitSyncOptions): InitOutput;

/**
 * Get the imports object for WebAssembly instantiation.
 *
 * @param memory - Optional shared memory to use instead of creating new
 * @returns The imports object for WebAssembly.Instance
 */
export function __wbg_get_imports(memory?: WebAssembly.Memory): WebAssembly.Imports;

/** The compiled WebAssembly module. Can be shared with workers. */
export const __wbg_wasm_module: WebAssembly.Module;

/** The shared WebAssembly memory. */
export const __wbg_memory: WebAssembly.Memory;
