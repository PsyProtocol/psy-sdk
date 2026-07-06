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
    /**
     * Compute sighash from an envelope + current header JSON.
     * Extracts nonce, user_id, and network_magic from the trace itself,
     * so JS doesn't need to parse the bincode payload.
     */
    compute_sighash_from_envelope_json(envelope_json: string, current_header_json: string): string;
    deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
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
     *   nullifier_json             - JSON array of 4 decimal strings
     *   note_secret_json      - JSON array of 4 decimal strings
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
    exec_shield_claim_deposit_json(pk_hash: string, nullifier_json: string, note_secret_json: string, token_address_u32x8_json: string, l2_token_contract_id_json: string, amount_u32x8_json: string, source_chain_index: string, deposit_index: string, deposit_root_json: string, deposit_siblings_json: string, random0: string, random1: string, contract_id: string): Promise<string>;
    generate_batch_claim_tx_trace_json(pk_hash: string, items_json: string): Promise<string>;
    generate_tx_trace_json(pk_hash: string, call_data_json: string): Promise<string>;
    get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
    get_random_keypair_json(): Promise<string>;
    get_result(id_str: string): Uint8Array;
    get_zk_public_key_json(private_key_str: string): Promise<string>;
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
     * Inputs (all u64 arrays as JSON arrays of decimal strings to avoid JS precision loss):
     *   pk_hash            - sender's ZK public key (hex QHashOut)
     *   owner_json         - receiver's shield address as JSON array of 4 decimal strings
     *   amount             - transfer amount (u64 as decimal string)
     *   note_secret_json - randomness used in commitment, JSON array of 4 decimal strings
     *   nullifier_secret_json - nullifier secret, JSON array of 4 decimal strings
     *   contract_id        - contract ID (u64 as decimal string)
     *   note_root_slot     - note root slot index (u64 as decimal string)
     *   checkpoint_id      - pre-submit checkpoint ID (u64 as decimal string, "0" = latest)
     *
     * Returns JSON matching NoteProofOutput.
     */
    prove_private_note_inclusion_json(pk_hash: string, owner_json: string, amount: string, note_secret_json: string, nullifier_secret_json: string, contract_id: string, note_root_slot: string, checkpoint_id: string): Promise<string>;
    /**
     * Stateless CFC step prove: reconstructs manager from JS-provided state.
     * Returns updated state. `leaf_records` with `insertion_proof` are
     * inside `proof_tree_meta`. Proof blobs returned as cfc_proof/ups_proof.
     */
    prove_trace_step_json(pk_hash: string, envelope_json: string, step_index: number, proof_tree_meta_json: string, last_step_info_json: string, current_header_json: string, previous_header_json: string): Promise<any>;
    prove_ups_start_job_json(pk_hash: string, envelope_json: string): Promise<string>;
    /**
     * Stateless ups_start prove: no manager persisted in WASM.
     * Returns all state JS needs for subsequent steps. `leaf_records` with
     * `insertion_proof` are inside `proof_tree_meta`. Proof blob returned
     * as `ups_proof` (Uint8Array) — JS stores it separately for finalize.
     */
    prove_ups_start_json(pk_hash: string, envelope_json: string): Promise<any>;
    prove_zksign_job_json(pk_hash: string, envelope_json: string): Promise<string>;
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
