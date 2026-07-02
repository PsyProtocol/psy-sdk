export function init_logging(): void;
export function main(): void;
export class WasmConstants {
    /**
     * @returns {string}
     */
    static get config_path(): string;
    /**
     * @returns {string}
     */
    static get coordinator_rpc_url(): string;
    /**
     * @returns {number}
     */
    static get coordinator_user_tree_height(): number;
    /**
     * @returns {string}
     */
    static get current_network(): string;
    /**
     * @returns {bigint}
     */
    static get deploy_contract_fee(): bigint;
    /**
     * Get all constants as a JSON string for easier JS consumption
     * @returns {string}
     */
    static getAllConstants(): string;
    /**
     * @returns {number}
     */
    static get global_user_tree_height(): number;
    /**
     * @returns {number}
     */
    static get group_realm_height(): number;
    /**
     * @returns {bigint}
     */
    static get guta_fee(): bigint;
    /**
     * @returns {string}
     */
    static get native_currency(): string;
    /**
     * @returns {number}
     */
    static get native_currency_decimal(): number;
    /**
     * @returns {string}
     */
    static get native_currency_name(): string;
    /**
     * @returns {string[]}
     */
    static get realm_rpc_urls(): string[];
    /**
     * @returns {number}
     */
    static get realm_user_tree_height(): number;
    /**
     * @returns {bigint}
     */
    static get register_user_fee(): bigint;
    /**
     * @returns {bigint}
     */
    static get users_per_realm(): bigint;
    __destroy_into_raw(): number | undefined;
    __wbg_ptr: number | undefined;
    free(): void;
}
export class WasmPsyConfig {
    static __wrap(ptr: any): any;
    /**
     * Create using builder pattern (for more complex configurations)
     * @returns {WasmPsyConfigBuilder}
     */
    static builder(): WasmPsyConfigBuilder;
    /**
     * @param {string} json
     */
    constructor(json: string);
    __destroy_into_raw(): any;
    __wbg_ptr: any;
    free(): void;
    /**
     * @returns {string}
     */
    currentNetworkName(): string;
    /**
     * @returns {string}
     */
    getCurrentNetwork(): string;
    /**
     * @param {string} network_name
     * @returns {string}
     */
    getNetworkJson(network_name: string): string;
    /**
     * @returns {string[]}
     */
    listNetworks(): string[];
    /**
     * @param {string} network_name
     */
    useNetwork(network_name: string): void;
}
/**
 * WASM Builder for flexible configuration in browser/JS environments
 */
export class WasmPsyConfigBuilder {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): any;
    __wbg_ptr: any;
    free(): void;
    /**
     * Build the configuration
     * @returns {WasmPsyConfig}
     */
    build(): WasmPsyConfig;
    /**
     * Set configuration from JSON string
     * @param {string} json
     * @returns {WasmPsyConfigBuilder}
     */
    json(json: string): WasmPsyConfigBuilder;
    /**
     * Set initial network to use
     * @param {string} network
     * @returns {WasmPsyConfigBuilder}
     */
    network(network: string): WasmPsyConfigBuilder;
}
export class WasmRpcServer {
    static __wrap(ptr: any): any;
    /**
     * @param {string} rpc_config_json
     */
    constructor(rpc_config_json: string);
    __destroy_into_raw(): number | undefined;
    __wbg_ptr: number | undefined;
    free(): void;
    /**
     * Inject an external PrivateNoteInclusion proof into the current session tree.
     * Returns JSON: { "leaf_index": u64, "siblings": [[u64;4]] }
     * @param {string} pk_hash
     * @param {string} note_proof_bincode_b64
     * @param {string | null} [note_proof_fingerprint_json]
     * @param {string | null} [note_verifier_data_json]
     * @returns {Promise<string>}
     */
    add_external_proof_json(pk_hash: string, note_proof_bincode_b64: string, note_proof_fingerprint_json?: string | null | undefined, note_verifier_data_json?: string | null | undefined): Promise<string>;
    /**
     * @param {string} private_key_str
     * @param {string} sign_type
     * @param {string | null} [fingerprint]
     * @returns {Promise<string>}
     */
    add_user(private_key_str: string, sign_type: string, fingerprint?: string | null | undefined): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} items_json
     * @returns {Promise<string>}
     */
    batch_claim_json(pk_hash: string, items_json: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} items_json
     * @returns {Promise<string>}
     */
    batch_claim_with_trace_json(pk_hash: string, items_json: string): Promise<string>;
    /**
     * Compute sighash from an envelope + current header JSON.
     * Extracts nonce, user_id, and network_magic from the trace itself,
     * so JS doesn't need to parse the bincode payload.
     * @param {string} envelope_json
     * @param {string} current_header_json
     * @returns {string}
     */
    compute_sighash_from_envelope_json(envelope_json: string, current_header_json: string): string;
    /**
     * @param {string} deployer
     * @param {string} circuit_defs_json
     * @returns {Promise<string>}
     */
    deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} claims_json
     * @returns {Promise<string>}
     */
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
     * @param {string} pk_hash
     * @param {string} note_proof_bincode_b64
     * @param {string} nullifier_json
     * @param {string} owner_json
     * @param {string} amount
     * @param {string} user_tree_root_json
     * @param {string} checkpoint_id
     * @param {string} note_root_slot
     * @param {string} contract_id
     * @param {string} random0
     * @param {string} random1
     * @param {string | null} [note_proof_fingerprint_json]
     * @param {string | null} [note_verifier_data_json]
     * @returns {Promise<string>}
     */
    exec_claim_with_external_proof_json(pk_hash: string, note_proof_bincode_b64: string, nullifier_json: string, owner_json: string, amount: string, user_tree_root_json: string, checkpoint_id: string, note_root_slot: string, contract_id: string, random0: string, random1: string, note_proof_fingerprint_json?: string | null | undefined, note_verifier_data_json?: string | null | undefined): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} call_data_json
     * @returns {Promise<string>}
     */
    exec_contract_call_json(pk_hash: string, call_data_json: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} call_data_json
     * @returns {Promise<string>}
     */
    exec_contract_call_with_trace_json(pk_hash: string, call_data_json: string): Promise<string>;
    /**
     * Atomic shield claim_deposit:
     * build ShieldDepositClaim proof -> start_session -> add_external_proof -> prove -> sign_and_submit.
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
     * @param {string} pk_hash
     * @param {string} nullifier_json
     * @param {string} note_secret_hash_json
     * @param {string} token_address_u32x8_json
     * @param {string} l2_token_contract_id_json
     * @param {string} amount_u32x8_json
     * @param {string} source_chain_index
     * @param {string} deposit_index
     * @param {string} deposit_root_json
     * @param {string} deposit_siblings_json
     * @param {string} random0
     * @param {string} random1
     * @param {string} contract_id
     * @returns {Promise<string>}
     */
    exec_shield_claim_deposit_json(pk_hash: string, nullifier_json: string, note_secret_hash_json: string, token_address_u32x8_json: string, l2_token_contract_id_json: string, amount_u32x8_json: string, source_chain_index: string, deposit_index: string, deposit_root_json: string, deposit_siblings_json: string, random0: string, random1: string, contract_id: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} items_json
     * @returns {Promise<string>}
     */
    generate_batch_claim_tx_trace_json(pk_hash: string, items_json: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} call_data_json
     * @returns {Promise<string>}
     */
    generate_tx_trace_json(pk_hash: string, call_data_json: string): Promise<string>;
    /**
     * @param {string} deployer
     * @param {string} circuit_defs_json
     * @returns {string}
     */
    get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
    /**
     * @returns {Promise<string>}
     */
    get_random_keypair_json(): Promise<string>;
    /**
     * @param {string} id_str
     * @returns {Uint8Array}
     */
    get_result(id_str: string): Uint8Array;
    /**
     * @param {string} private_key_str
     * @returns {Promise<string>}
     */
    get_zk_public_key_json(private_key_str: string): Promise<string>;
    /**
     * Stateless external proof insertion: inject a private_note_inclusion or
     * shield_deposit_claim proof into the proof tree. No baton/header changes.
     * Returns the updated `proof_tree_meta` with the new leaf's metadata
     * appended to `leaf_records`.
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @param {string} proof_tree_meta_json
     * @param {string} last_step_info_json
     * @param {string} current_header_json
     * @param {string} previous_header_json
     * @param {string} external_fingerprint
     * @param {Uint8Array} external_proof
     * @returns {Promise<any>}
     */
    insert_external_proof_json(pk_hash: string, envelope_json: string, proof_tree_meta_json: string, last_step_info_json: string, current_header_json: string, previous_header_json: string, external_fingerprint: string, external_proof: Uint8Array): Promise<any>;
    /**
     * @param {string} message
     * @returns {string}
     */
    ping(message: string): string;
    /**
     * @param {string} envelope_json
     * @returns {Promise<string>}
     */
    prepare_trace_proof_schedule_json(envelope_json: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @param {string} schedule_json
     * @param {number} step_index
     * @returns {Promise<string>}
     */
    prove_cfc_job_with_schedule_step_json(pk_hash: string, envelope_json: string, schedule_json: string, step_index: number): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} contract_call_json
     * @returns {Promise<string>}
     */
    prove_contract_call_json(pk_hash: string, contract_call_json: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} contract_calls_json
     * @returns {Promise<string>}
     */
    prove_contract_calls_json(pk_hash: string, contract_calls_json: string): Promise<string>;
    /**
     * Stateless end-cap prove: reconstructs all leaf_proofs from JS-provided records,
     * adds ZkSign leaf, runs finalize_tree. Takes external signature proof.
     * `all_proof_blobs` are bincode-serialized `ProofWithPublicInputs` for
     * each leaf in insertion order (from trace cfc_proof/ups_proof).
     * `proof_tree_meta` must contain `leaf_records` with `insertion_proof`.
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @param {string} proof_tree_meta_json
     * @param {string} last_step_info_json
     * @param {Uint8Array[]} all_proof_blobs
     * @param {Uint8Array} signature_proof
     * @returns {Promise<any>}
     */
    prove_end_cap_proof_json(pk_hash: string, envelope_json: string, proof_tree_meta_json: string, last_step_info_json: string, all_proof_blobs: Uint8Array[], signature_proof: Uint8Array): Promise<any>;
    /**
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @param {string} schedule_json
     * @param {string[]} output_jsons
     * @returns {Promise<string>}
     */
    prove_endcap_job_from_output_jsons_json(pk_hash: string, envelope_json: string, schedule_json: string, output_jsons: string[]): Promise<string>;
    /**
     * @param {string} envelope_json
     * @param {number} step_index
     * @returns {Promise<string>}
     */
    prove_external_proof_job_json(envelope_json: string, step_index: number): Promise<string>;
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
     * @param {string} pk_hash
     * @param {string} owner_json
     * @param {string} amount
     * @param {string} note_secret_hash_json
     * @param {string} nullifier_secret_json
     * @param {string} contract_id
     * @param {string} note_root_slot
     * @param {string} checkpoint_id
     * @returns {Promise<string>}
     */
    prove_private_note_inclusion_json(pk_hash: string, owner_json: string, amount: string, note_secret_hash_json: string, nullifier_secret_json: string, contract_id: string, note_root_slot: string, checkpoint_id: string): Promise<string>;
    /**
     * Stateless CFC step prove: reconstructs manager from JS-provided state.
     * Returns updated state. `leaf_records` with `insertion_proof` are
     * inside `proof_tree_meta`. Proof blobs returned as cfc_proof/ups_proof.
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @param {number} step_index
     * @param {string} proof_tree_meta_json
     * @param {string} last_step_info_json
     * @param {string} current_header_json
     * @param {string} previous_header_json
     * @returns {Promise<any>}
     */
    prove_trace_step_json(pk_hash: string, envelope_json: string, step_index: number, proof_tree_meta_json: string, last_step_info_json: string, current_header_json: string, previous_header_json: string): Promise<any>;
    /**
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @returns {Promise<string>}
     */
    prove_ups_start_job_json(pk_hash: string, envelope_json: string): Promise<string>;
    /**
     * Stateless ups_start prove: no manager persisted in WASM.
     * Returns all state JS needs for subsequent steps. `leaf_records` with
     * `insertion_proof` are inside `proof_tree_meta`. Proof blob returned
     * as `ups_proof` (Uint8Array) — JS stores it separately for finalize.
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @returns {Promise<any>}
     */
    prove_ups_start_json(pk_hash: string, envelope_json: string): Promise<any>;
    /**
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @returns {Promise<string>}
     */
    prove_zksign_job_json(pk_hash: string, envelope_json: string): Promise<string>;
    /**
     * @param {BigUint64Array} allowed_contract_ids
     * @param {BigUint64Array} allowed_method_ids
     * @param {bigint} expected_tx_count
     * @returns {Promise<string>}
     */
    register_sd_key_circuit(allowed_contract_ids: BigUint64Array, allowed_method_ids: BigUint64Array, expected_tx_count: bigint): Promise<string>;
    /**
     * @param {string} private_key_str
     * @param {string} sign_type
     * @param {string | null} [fingerprint]
     * @returns {Promise<string>}
     */
    register_user(private_key_str: string, sign_type: string, fingerprint?: string | null | undefined): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string | null} [sign_data]
     * @returns {Promise<string>}
     */
    sign_and_submit(pk_hash: string, sign_data?: string | null | undefined): Promise<string>;
    /**
     * Sign a sighash with the wallet's private key and return the signature
     * proof as bincode bytes (Uint8Array). Used by the step proving path:
     * JS calls `compute_sighash_from_envelope_json` → `sign_sighash_json` →
     * passes the result to `prove_end_cap_proof_json`.
     *
     * NOTE: This still uses the wallet's in-WASM private key. Full signer
     * externalisation (Phase 2) would move this to JS.
     * @param {string} pk_hash
     * @param {string} sighash_json
     * @param {string | null} [envelope_json]
     * @param {string | null} [current_header_json]
     * @returns {Promise<Uint8Array>}
     */
    sign_sighash_json(pk_hash: string, sighash_json: string, envelope_json?: string | null | undefined, current_header_json?: string | null | undefined): Promise<Uint8Array>;
    /**
     * @param {string} pk_hash
     * @returns {Promise<string>}
     */
    start_session(pk_hash: string): Promise<string>;
    /**
     * Submit a pre-proven end-cap proof (RPC only, no proving).
     * @param {string} envelope_json
     * @param {Uint8Array} end_cap_proof
     * @returns {Promise<string>}
     */
    submit_end_cap_json(envelope_json: string, end_cap_proof: Uint8Array): Promise<string>;
    /**
     * @param {string} envelope_json
     * @param {string} endcap_output_json
     * @returns {Promise<string>}
     */
    submit_endcap_job_json(envelope_json: string, endcap_output_json: string): Promise<string>;
    /**
     * @param {string} envelope_json
     * @returns {string}
     */
    trace_proof_job_step_indices_json(envelope_json: string): string;
}
export function initSync(module: any, memory: any): any;
declare function __wbg_init(module_or_path: any, memory: any): Promise<any>;
export { __wbg_init as default };
//# sourceMappingURL=psy_prover.d.ts.map