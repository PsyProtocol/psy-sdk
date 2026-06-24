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
    /**
     * MODE-A (personal_sign) claim: submit a claim batch authorized SOLELY by a
     * MetaMask `personal_sign`. Mirrors `claim_batch_with_external_signature`,
     * installing an `ExternalEthPersonalSignUser`. Prime with `get_claim_sig_hash`
     * first; the wallet `personal_sign`s the value it returns.
     */
    claim_batch_with_external_eth_personal_signature(pk_hash: string, _claims_json: string, signature_hex: string): Promise<string>;
    /**
     * MODE-A step 3 (claim): submit a claim batch authorized SOLELY by an
     * external `eth_sign` signature (no held key).
     *
     * `signature_hex` is the 97-byte `compressed_pubkey ‖ r ‖ s` an `eth_sign`
     * produced over the sighash from `get_claim_sig_hash`. Reuses the session
     * primed by `get_claim_sig_hash`, recomputes the sighash, binds it as the
     * signature's `message`, registers an `ExternalSecp256K1User` for this
     * `pk_hash`, and runs the UNCHANGED `sign_and_submit` (the same end-cap
     * `claim_batch` itself runs). Returns the tx end-user-leaf hash.
     */
    claim_batch_with_external_signature(pk_hash: string, _claims_json: string, signature_hex: string): Promise<string>;
    deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
    exec_claim_batch_json(pk_hash: string, claims_json: string): Promise<string>;
    exec_claim_batch_without_proof_json(pk_hash: string, claims_json: string): Promise<string>;
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
     * MODE-A (personal_sign) contract call: submit a public contract call
     * authorized SOLELY by a MetaMask `personal_sign`. Mirrors
     * `exec_contract_call_with_external_signature`, installing an
     * `ExternalEthPersonalSignUser`. Prime the session with `get_sig_hash` first;
     * the wallet `personal_sign`s the value it returns.
     */
    exec_contract_call_with_external_eth_personal_signature(pk_hash: string, call_data_json: string, signature_hex: string): Promise<string>;
    /**
     * MODE-A step 3 (contract call): submit a public contract call authorized
     * SOLELY by an external `eth_sign` signature (no held key).
     *
     * `signature_hex` is the 97-byte `compressed_pubkey(33) ‖ r(32) ‖ s(32)` an
     * `eth_sign` / `sign_prehash` produces over the sighash from `get_sig_hash`.
     * Reuses the session primed by `get_sig_hash`, recomputes the sighash,
     * binds it as the signature's `message`, registers an `ExternalSecp256K1User`
     * for this `pk_hash`, and runs the UNCHANGED `sign_and_submit`. Returns the
     * tx end-user-leaf hash.
     */
    exec_contract_call_with_external_signature(pk_hash: string, call_data_json: string, signature_hex: string): Promise<string>;
    exec_contract_call_without_proof_json(pk_hash: string, call_data_json: string): Promise<string>;
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
    /**
     * MODE-A step 1 (claim): drive a claim batch up to (but not through) the
     * authorization signature, and return the 32-byte Psy session sighash (hex)
     * the external wallet must `eth_sign`.
     *
     * Runs `prove_claim_batch` (start_session + per-item claim proofs + burn fee
     * — the same prefix `claim_batch` runs, minus sign), then reads the
     * deterministic `get_sighash(PSY_NETWORK_MAGIC, nonce)`. The session is left
     * primed; pair this with `claim_batch_with_external_signature`.
     */
    get_claim_sig_hash(pk_hash: string, claims_json: string): Promise<string>;
    get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
    get_random_keypair_json(): Promise<string>;
    get_result(id_str: string): Uint8Array;
    /**
     * MODE-A step 1 (contract call): drive a public contract call up to (but not
     * through) the authorization signature, and return the 32-byte Psy session
     * sighash (hex) the external wallet (MetaMask) must `eth_sign`.
     *
     * Runs `start_session` + `prove_contract_call` (the same prefix
     * `exec_contract_call` runs), then reads the deterministic
     * `get_sighash(PSY_NETWORK_MAGIC, nonce)` from the session manager. The
     * session is left primed; pair this with
     * `exec_contract_call_with_external_signature`, which reuses the same state
     * and asserts the sighash is unchanged before submit (a stale-nonce guard).
     */
    get_sig_hash(pk_hash: string, call_data_json: string): Promise<string>;
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
    /**
     * MODE-A (personal_sign) register: register a secp256k1 PUBLIC key as a Psy
     * account under the EIP-191 signature type, no held key. Mirrors
     * `register_user_with_external_signature`, installing an
     * `ExternalEthPersonalSignUser`.
     */
    register_user_with_external_eth_personal_signature(public_key_hex: string, signature_hex: string): Promise<string>;
    /**
     * MODE-A core primitive: register a secp256k1 PUBLIC key as a Psy account
     * WITHOUT a held private key, authorized by an external `eth_sign`.
     *
     * `public_key_hex` is the SEC1-compressed public key (33 bytes / 66 hex
     * chars). `signature_hex` is the 97-byte `compressed_pubkey ‖ r ‖ s` an
     * `eth_sign` produced over ANY well-formed Psy sighash — the registration
     * itself does not consume a session sighash, but supplying a real signature
     * proves possession of the key and keeps the payload format uniform across
     * every Mode-A method. `public_key_hex` must equal the first 33 bytes of
     * `signature_hex`.
     *
     * Installs an `ExternalSecp256K1User` for the derived `pk_hash` and submits
     * the SAME `register_user` request the held-key secp256k1 path submits, then
     * polls `get_user_ids_for_public_key` for the assigned `user_id`. Returns
     * JSON `{ "pk_hash": "...", "user_id": "..." | null }` — `user_id` is `null`
     * when registration has not yet landed on-chain (the bridge should keep
     * polling `get_user_ids_for_public_key`).
     */
    register_user_with_external_signature(public_key_hex: string, signature_hex: string): Promise<string>;
}

export function init_logging(): void;

export function main(): void;
