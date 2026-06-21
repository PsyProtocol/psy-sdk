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
     * @returns {Promise<string>}
     */
    add_external_proof_json(pk_hash: string, note_proof_bincode_b64: string): Promise<string>;
    /**
     * @param {string} private_key_str
     * @param {string} sign_type
     * @param {string | null} [sdk_key_fingerprint]
     * @returns {Promise<string>}
     */
    add_user(private_key_str: string, sign_type: string, sdk_key_fingerprint?: string | null | undefined): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} items_json
     * @returns {Promise<string>}
     */
    batch_claim_json(pk_hash: string, items_json: string): Promise<string>;
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
     * @returns {Promise<string>}
     */
    exec_claim_with_external_proof_json(pk_hash: string, note_proof_bincode_b64: string, nullifier_json: string, owner_json: string, amount: string, user_tree_root_json: string, checkpoint_id: string, note_root_slot: string, contract_id: string, random0: string, random1: string): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string} call_data_json
     * @returns {Promise<string>}
     */
    exec_contract_call_json(pk_hash: string, call_data_json: string): Promise<string>;
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
     * @param {string} message
     * @returns {string}
     */
    ping(message: string): string;
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
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @returns {Promise<string>}
     */
    prove_tx_trace_json(pk_hash: string, envelope_json: string): Promise<string>;
    /**
     * @param {BigUint64Array} allowed_contract_ids
     * @param {BigUint64Array} allowed_method_ids
     * @param {bigint} expected_tx_count
     * @returns {Promise<string>}
     */
    register_sdk_key_circuit(allowed_contract_ids: BigUint64Array, allowed_method_ids: BigUint64Array, expected_tx_count: bigint): Promise<string>;
    /**
     * @param {string} private_key_str
     * @param {string} sign_type
     * @param {string | null} [sdk_key_fingerprint]
     * @returns {Promise<string>}
     */
    register_user(private_key_str: string, sign_type: string, sdk_key_fingerprint?: string | null | undefined): Promise<string>;
    /**
     * @param {string} pk_hash
     * @param {string | null} [sign_data]
     * @returns {Promise<string>}
     */
    sign_and_submit(pk_hash: string, sign_data?: string | null | undefined): Promise<string>;
    /**
     * @param {string} pk_hash
     * @returns {Promise<string>}
     */
    start_session(pk_hash: string): Promise<string>;
}
export function initSync(module: any): any;
declare function __wbg_init(module_or_path: any): Promise<any>;
export { __wbg_init as default };
//# sourceMappingURL=psy_prover.d.ts.map