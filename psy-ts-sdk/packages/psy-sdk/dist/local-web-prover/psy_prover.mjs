/* @ts-self-types="./psy_prover.d.ts" */
class WasmConstants {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmConstantsFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmconstants_free(ptr, 0);
    }
    /**
     * @returns {string}
     */
    static get config_path() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmconstants_config_path();
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        }
        finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    static get coordinator_rpc_url() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmconstants_coordinator_rpc_url();
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        }
        finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    static get coordinator_user_tree_height() {
        const ret = wasm.wasmconstants_coordinator_user_tree_height();
        return ret;
    }
    /**
     * @returns {string}
     */
    static get current_network() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmconstants_current_network();
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        }
        finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {bigint}
     */
    static get deploy_contract_fee() {
        const ret = wasm.wasmconstants_deploy_contract_fee();
        return BigInt.asUintN(64, ret);
    }
    /**
     * Get all constants as a JSON string for easier JS consumption
     * @returns {string}
     */
    static getAllConstants() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmconstants_getAllConstants();
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0;
                len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        }
        finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    static get global_user_tree_height() {
        const ret = wasm.wasmconstants_global_user_tree_height();
        return ret;
    }
    /**
     * @returns {number}
     */
    static get group_realm_height() {
        const ret = wasm.wasmconstants_group_realm_height();
        return ret;
    }
    /**
     * @returns {bigint}
     */
    static get guta_fee() {
        const ret = wasm.wasmconstants_guta_fee();
        return BigInt.asUintN(64, ret);
    }
    /**
     * @returns {string}
     */
    static get native_currency() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmconstants_native_currency();
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        }
        finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    static get native_currency_decimal() {
        const ret = wasm.wasmconstants_native_currency_decimal();
        return ret;
    }
    /**
     * @returns {string}
     */
    static get native_currency_name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmconstants_native_currency_name();
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        }
        finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string[]}
     */
    static get realm_rpc_urls() {
        const ret = wasm.wasmconstants_realm_rpc_urls();
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    static get realm_user_tree_height() {
        const ret = wasm.wasmconstants_realm_user_tree_height();
        return ret;
    }
    /**
     * @returns {bigint}
     */
    static get register_user_fee() {
        const ret = wasm.wasmconstants_register_user_fee();
        return BigInt.asUintN(64, ret);
    }
    /**
     * @returns {bigint}
     */
    static get users_per_realm() {
        const ret = wasm.wasmconstants_users_per_realm();
        return BigInt.asUintN(64, ret);
    }
}
if (Symbol.dispose)
    WasmConstants.prototype[Symbol.dispose] = WasmConstants.prototype.free;
class WasmPsyConfig {
    static __wrap(ptr) {
        const obj = Object.create(WasmPsyConfig.prototype);
        obj.__wbg_ptr = ptr;
        WasmPsyConfigFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPsyConfigFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpsyconfig_free(ptr, 0);
    }
    /**
     * Create using builder pattern (for more complex configurations)
     * @returns {WasmPsyConfigBuilder}
     */
    static builder() {
        const ret = wasm.wasmpsyconfig_builder();
        return WasmPsyConfigBuilder.__wrap(ret);
    }
    /**
     * @returns {string}
     */
    currentNetworkName() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpsyconfig_currentNetworkName(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        }
        finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    getCurrentNetwork() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmpsyconfig_getCurrentNetwork(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0;
                len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        }
        finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} network_name
     * @returns {string}
     */
    getNetworkJson(network_name) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(network_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmpsyconfig_getNetworkJson(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0;
                len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        }
        finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @returns {string[]}
     */
    listNetworks() {
        const ret = wasm.wasmpsyconfig_listNetworks(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {string} json
     */
    constructor(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpsyconfig_new(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmPsyConfigFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {string} network_name
     */
    useNetwork(network_name) {
        const ptr0 = passStringToWasm0(network_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpsyconfig_useNetwork(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose)
    WasmPsyConfig.prototype[Symbol.dispose] = WasmPsyConfig.prototype.free;
/**
 * WASM Builder for flexible configuration in browser/JS environments
 */
class WasmPsyConfigBuilder {
    static __wrap(ptr) {
        const obj = Object.create(WasmPsyConfigBuilder.prototype);
        obj.__wbg_ptr = ptr;
        WasmPsyConfigBuilderFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPsyConfigBuilderFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpsyconfigbuilder_free(ptr, 0);
    }
    /**
     * Build the configuration
     * @returns {WasmPsyConfig}
     */
    build() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmpsyconfigbuilder_build(ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmPsyConfig.__wrap(ret[0]);
    }
    /**
     * Set configuration from JSON string
     * @param {string} json
     * @returns {WasmPsyConfigBuilder}
     */
    json(json) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpsyconfigbuilder_json(ptr, ptr0, len0);
        return WasmPsyConfigBuilder.__wrap(ret);
    }
    /**
     * Set initial network to use
     * @param {string} network
     * @returns {WasmPsyConfigBuilder}
     */
    network(network) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passStringToWasm0(network, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpsyconfigbuilder_network(ptr, ptr0, len0);
        return WasmPsyConfigBuilder.__wrap(ret);
    }
    constructor() {
        const ret = wasm.wasmpsyconfigbuilder_new();
        this.__wbg_ptr = ret;
        WasmPsyConfigBuilderFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose)
    WasmPsyConfigBuilder.prototype[Symbol.dispose] = WasmPsyConfigBuilder.prototype.free;
class WasmRpcServer {
    static __wrap(ptr) {
        const obj = Object.create(WasmRpcServer.prototype);
        obj.__wbg_ptr = ptr;
        WasmRpcServerFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmRpcServerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmrpcserver_free(ptr, 0);
    }
    /**
     * Inject an external PrivateNoteInclusion proof into the current session tree.
     * Returns JSON: { "leaf_index": u64, "siblings": [[u64;4]] }
     * @param {string} pk_hash
     * @param {string} note_proof_bincode_b64
     * @returns {Promise<string>}
     */
    add_external_proof_json(pk_hash, note_proof_bincode_b64) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(note_proof_bincode_b64, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_add_external_proof_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {string} private_key_str
     * @param {string} sign_type
     * @param {string | null} [sdk_key_fingerprint]
     * @returns {Promise<string>}
     */
    add_user(private_key_str, sign_type, sdk_key_fingerprint) {
        const ptr0 = passStringToWasm0(private_key_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(sign_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(sdk_key_fingerprint) ? 0 : passStringToWasm0(sdk_key_fingerprint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_add_user(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string} items_json
     * @returns {Promise<string>}
     */
    batch_claim_json(pk_hash, items_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(items_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_batch_claim_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {string} deployer
     * @param {string} circuit_defs_json
     * @returns {Promise<string>}
     */
    deploy_contract_json(deployer, circuit_defs_json) {
        const ptr0 = passStringToWasm0(deployer, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(circuit_defs_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_deploy_contract_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string} claims_json
     * @returns {Promise<string>}
     */
    exec_claim_batch_json(pk_hash, claims_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(claims_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_exec_claim_batch_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
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
    exec_claim_with_external_proof_json(pk_hash, note_proof_bincode_b64, nullifier_json, owner_json, amount, user_tree_root_json, checkpoint_id, note_root_slot, contract_id, random0, random1) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(note_proof_bincode_b64, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(nullifier_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(owner_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(user_tree_root_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ptr6 = passStringToWasm0(checkpoint_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len6 = WASM_VECTOR_LEN;
        const ptr7 = passStringToWasm0(note_root_slot, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len7 = WASM_VECTOR_LEN;
        const ptr8 = passStringToWasm0(contract_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len8 = WASM_VECTOR_LEN;
        const ptr9 = passStringToWasm0(random0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len9 = WASM_VECTOR_LEN;
        const ptr10 = passStringToWasm0(random1, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len10 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_exec_claim_with_external_proof_json(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, ptr7, len7, ptr8, len8, ptr9, len9, ptr10, len10);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string} call_data_json
     * @returns {Promise<string>}
     */
    exec_contract_call_json(pk_hash, call_data_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(call_data_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_exec_contract_call_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
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
    exec_shield_claim_deposit_json(pk_hash, nullifier_json, note_secret_hash_json, token_address_u32x8_json, l2_token_contract_id_json, amount_u32x8_json, source_chain_index, deposit_index, deposit_root_json, deposit_siblings_json, random0, random1, contract_id) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(nullifier_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(note_secret_hash_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(token_address_u32x8_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(l2_token_contract_id_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(amount_u32x8_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ptr6 = passStringToWasm0(source_chain_index, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len6 = WASM_VECTOR_LEN;
        const ptr7 = passStringToWasm0(deposit_index, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len7 = WASM_VECTOR_LEN;
        const ptr8 = passStringToWasm0(deposit_root_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len8 = WASM_VECTOR_LEN;
        const ptr9 = passStringToWasm0(deposit_siblings_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len9 = WASM_VECTOR_LEN;
        const ptr10 = passStringToWasm0(random0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len10 = WASM_VECTOR_LEN;
        const ptr11 = passStringToWasm0(random1, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len11 = WASM_VECTOR_LEN;
        const ptr12 = passStringToWasm0(contract_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len12 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_exec_shield_claim_deposit_json(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, ptr7, len7, ptr8, len8, ptr9, len9, ptr10, len10, ptr11, len11, ptr12, len12);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string} items_json
     * @returns {Promise<string>}
     */
    generate_batch_claim_tx_trace_json(pk_hash, items_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(items_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_generate_batch_claim_tx_trace_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string} call_data_json
     * @returns {Promise<string>}
     */
    generate_tx_trace_json(pk_hash, call_data_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(call_data_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_generate_tx_trace_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {string} deployer
     * @param {string} circuit_defs_json
     * @returns {string}
     */
    get_deploy_contract_cmd_json(deployer, circuit_defs_json) {
        let deferred4_0;
        let deferred4_1;
        try {
            const ptr0 = passStringToWasm0(deployer, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(circuit_defs_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmrpcserver_get_deploy_contract_cmd_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0;
                len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        }
        finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * @returns {Promise<string>}
     */
    get_random_keypair_json() {
        const ret = wasm.wasmrpcserver_get_random_keypair_json(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {string} id_str
     * @returns {Uint8Array}
     */
    get_result(id_str) {
        const ptr0 = passStringToWasm0(id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_get_result(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v2;
    }
    /**
     * @param {string} private_key_str
     * @returns {Promise<string>}
     */
    get_zk_public_key_json(private_key_str) {
        const ptr0 = passStringToWasm0(private_key_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_get_zk_public_key_json(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * @param {string} rpc_config_json
     */
    constructor(rpc_config_json) {
        const ptr0 = passStringToWasm0(rpc_config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_new(ptr0, len0);
        return ret;
    }
    /**
     * @param {string} message
     * @returns {string}
     */
    ping(message) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmrpcserver_ping(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0;
                len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        }
        finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} pk_hash
     * @param {string} contract_call_json
     * @returns {Promise<string>}
     */
    prove_contract_call_json(pk_hash, contract_call_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(contract_call_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_prove_contract_call_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string} contract_calls_json
     * @returns {Promise<string>}
     */
    prove_contract_calls_json(pk_hash, contract_calls_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(contract_calls_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_prove_contract_calls_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
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
    prove_private_note_inclusion_json(pk_hash, owner_json, amount, note_secret_hash_json, nullifier_secret_json, contract_id, note_root_slot, checkpoint_id) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(owner_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(note_secret_hash_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(nullifier_secret_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(contract_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ptr6 = passStringToWasm0(note_root_slot, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len6 = WASM_VECTOR_LEN;
        const ptr7 = passStringToWasm0(checkpoint_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len7 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_prove_private_note_inclusion_json(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6, ptr7, len7);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string} envelope_json
     * @returns {Promise<string>}
     */
    prove_tx_trace_json(pk_hash, envelope_json) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(envelope_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_prove_tx_trace_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {BigUint64Array} allowed_contract_ids
     * @param {BigUint64Array} allowed_method_ids
     * @param {bigint} expected_tx_count
     * @returns {Promise<string>}
     */
    register_sdk_key_circuit(allowed_contract_ids, allowed_method_ids, expected_tx_count) {
        const ptr0 = passArray64ToWasm0(allowed_contract_ids, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray64ToWasm0(allowed_method_ids, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_register_sdk_key_circuit(this.__wbg_ptr, ptr0, len0, ptr1, len1, expected_tx_count);
        return ret;
    }
    /**
     * @param {string} private_key_str
     * @param {string} sign_type
     * @param {string | null} [sdk_key_fingerprint]
     * @returns {Promise<string>}
     */
    register_user(private_key_str, sign_type, sdk_key_fingerprint) {
        const ptr0 = passStringToWasm0(private_key_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(sign_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(sdk_key_fingerprint) ? 0 : passStringToWasm0(sdk_key_fingerprint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_register_user(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @param {string | null} [sign_data]
     * @returns {Promise<string>}
     */
    sign_and_submit(pk_hash, sign_data) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(sign_data) ? 0 : passStringToWasm0(sign_data, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_sign_and_submit(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * @param {string} pk_hash
     * @returns {Promise<string>}
     */
    start_session(pk_hash) {
        const ptr0 = passStringToWasm0(pk_hash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrpcserver_start_session(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
}
if (Symbol.dispose)
    WasmRpcServer.prototype[Symbol.dispose] = WasmRpcServer.prototype.free;
function init_logging() {
    wasm.init_logging();
}
function main() {
    wasm.main();
}
function __wbg_get_imports(memory) {
    const import0 = {
        __proto__: null,
        __wbg_Error_fdd633d4bb5dd76a: function (arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg___wbindgen_debug_string_8a447059637473e2: function (arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_acc5528be2b923f2: function (arg0) {
            const ret = typeof (arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_0beba4a1980d3eea: function (arg0) {
            const val = arg0;
            const ret = typeof (val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_1fca8072260dd261: function (arg0) {
            const ret = typeof (arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_721f8decd50c87a3: function (arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_memory_9751d9a3017e7c25: function () {
            const ret = wasm.memory;
            return ret;
        },
        __wbg___wbindgen_rethrow_858623e73c3311dc: function (arg0) {
            throw arg0;
        },
        __wbg___wbindgen_string_get_71bb4348194e31f0: function (arg0, arg1) {
            const obj = arg1;
            const ret = typeof (obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_ea4887a5f8f9a9db: function (arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_33c39e13d73b25f6: function (arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_abort_6e6ea7d259504afc: function (arg0) {
            arg0.abort();
        },
        __wbg_abort_9e39323f373e2585: function (arg0, arg1) {
            arg0.abort(arg1);
        },
        __wbg_append_912a8705e9b6a483: function () {
            return handleError(function (arg0, arg1, arg2, arg3, arg4) {
                arg0.append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            }, arguments);
        },
        __wbg_arrayBuffer_ff96d08b7b6be32e: function () {
            return handleError(function (arg0) {
                const ret = arg0.arrayBuffer();
                return ret;
            }, arguments);
        },
        __wbg_async_4ec36f08efecafdc: function (arg0) {
            const ret = arg0.async;
            return ret;
        },
        __wbg_buffer_49b4f592d8036785: function (arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_call_0e855b388e315e17: function () {
            return handleError(function (arg0, arg1, arg2, arg3) {
                const ret = arg0.call(arg1, arg2, arg3);
                return ret;
            }, arguments);
        },
        __wbg_call_5575218572ead796: function () {
            return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.call(arg1, arg2);
                return ret;
            }, arguments);
        },
        __wbg_call_8e98ed2f3c86c4b5: function () {
            return handleError(function (arg0, arg1) {
                const ret = arg0.call(arg1);
                return ret;
            }, arguments);
        },
        __wbg_clearTimeout_6b8d9a38b9263d65: function (arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        },
        __wbg_crypto_38df2bab126b63dc: function (arg0) {
            const ret = arg0.crypto;
            return ret;
        },
        __wbg_data_23c14f7d1102d077: function (arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_debug_7271beced8b71cd4: function (arg0, arg1, arg2, arg3) {
            console.debug(arg0, arg1, arg2, arg3);
        },
        __wbg_done_b62d4a7d2286852a: function (arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_error_50f60c611a3dcf64: function (arg0, arg1, arg2, arg3) {
            console.error(arg0, arg1, arg2, arg3);
        },
        __wbg_error_933f449d72fef598: function (arg0) {
            console.error(arg0);
        },
        __wbg_error_a6fa202b58aa1cd3: function (arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            }
            finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_fetch_9dad4fe911207b37: function (arg0) {
            const ret = fetch(arg0);
            return ret;
        },
        __wbg_fetch_db87be8a748781a2: function (arg0, arg1) {
            const ret = arg0.fetch(arg1);
            return ret;
        },
        __wbg_getRandomValues_b2176991427f6db8: function () {
            return handleError(function (arg0) {
                globalThis.crypto.getRandomValues(arg0);
            }, arguments);
        },
        __wbg_getRandomValues_c44a50d8cfdaebeb: function () {
            return handleError(function (arg0, arg1) {
                arg0.getRandomValues(arg1);
            }, arguments);
        },
        __wbg_get_9a29be2cb383ed9a: function () {
            return handleError(function (arg0, arg1) {
                const ret = Reflect.get(arg0, arg1);
                return ret;
            }, arguments);
        },
        __wbg_get_dddb90ff5d27a080: function () {
            return handleError(function (arg0, arg1) {
                const ret = Reflect.get(arg0, arg1);
                return ret;
            }, arguments);
        },
        __wbg_has_4f060fe202ad7e87: function () {
            return handleError(function (arg0, arg1) {
                const ret = Reflect.has(arg0, arg1);
                return ret;
            }, arguments);
        },
        __wbg_headers_d9123c649c85d441: function (arg0) {
            const ret = arg0.headers;
            return ret;
        },
        __wbg_info_a392cd5b7536cfb5: function (arg0, arg1, arg2, arg3) {
            console.info(arg0, arg1, arg2, arg3);
        },
        __wbg_instanceof_Response_79948c98d1d2ba75: function (arg0) {
            let result;
            try {
                result = arg0 instanceof Response;
            }
            catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_iterator_cc47ba25a2be735a: function () {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_length_589238bdcf171f0e: function (arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_log_17a3e9a5cbb91ef7: function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.log(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7));
            }
            finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_log_d282446d03691e72: function (arg0, arg1, arg2, arg3) {
            console.log(arg0, arg1, arg2, arg3);
        },
        __wbg_log_e885b89e7e480a2f: function (arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.log(getStringFromWasm0(arg0, arg1));
            }
            finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_mark_0279c5d75168b5b8: function (arg0, arg1) {
            performance.mark(getStringFromWasm0(arg0, arg1));
        },
        __wbg_measure_c9b58ac538b3e2f7: function () {
            return handleError(function (arg0, arg1, arg2, arg3) {
                let deferred0_0;
                let deferred0_1;
                let deferred1_0;
                let deferred1_1;
                try {
                    deferred0_0 = arg0;
                    deferred0_1 = arg1;
                    deferred1_0 = arg2;
                    deferred1_1 = arg3;
                    performance.measure(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
                }
                finally {
                    wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
                    wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
                }
            }, arguments);
        },
        __wbg_msCrypto_bd5a034af96bcba6: function (arg0) {
            const ret = arg0.msCrypto;
            return ret;
        },
        __wbg_new_10e2f2ad134f940f: function () {
            return handleError(function () {
                const ret = new Headers();
                return ret;
            }, arguments);
        },
        __wbg_new_227d7c05414eb861: function () {
            const ret = new Error();
            return ret;
        },
        __wbg_new_2e117a478906f062: function () {
            const ret = new Object();
            return ret;
        },
        __wbg_new_476e05fb84d8e4f3: function (arg0) {
            const ret = new Int32Array(arg0);
            return ret;
        },
        __wbg_new_51233fa2a760b272: function () {
            return handleError(function () {
                const ret = new AbortController();
                return ret;
            }, arguments);
        },
        __wbg_new_81880fb5002cb255: function (arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_f85beb941dc6d8aa: function (arg0, arg1) {
            try {
                var state0 = { a: arg0, b: arg1 };
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h18544ad86c9831de(a, state0.b, arg0, arg1);
                    }
                    finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            }
            finally {
                state0.a = 0;
            }
        },
        __wbg_new_from_slice_543b875b27789a8f: function (arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_00a409eb4ec4f2d9: function (arg0, arg1) {
            try {
                var state0 = { a: arg0, b: arg1 };
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h18544ad86c9831de(a, state0.b, arg0, arg1);
                    }
                    finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            }
            finally {
                state0.a = 0;
            }
        },
        __wbg_new_with_length_9b650f44b5c44a4e: function (arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return ret;
        },
        __wbg_new_with_str_and_init_5b299538bdeeec64: function () {
            return handleError(function (arg0, arg1, arg2) {
                const ret = new Request(getStringFromWasm0(arg0, arg1), arg2);
                return ret;
            }, arguments);
        },
        __wbg_new_worker_e68fc8188cc230b5: function (arg0, arg1) {
            const ret = new Worker(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_next_0c4066e251d2eff9: function () {
            return handleError(function (arg0) {
                const ret = arg0.next();
                return ret;
            }, arguments);
        },
        __wbg_next_402fa10b59ab20c3: function (arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_node_84ea875411254db1: function (arg0) {
            const ret = arg0.node;
            return ret;
        },
        __wbg_now_d2e0afbad4edbe82: function () {
            const ret = Date.now();
            return ret;
        },
        __wbg_now_e7c6795a7f81e10f: function (arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_of_8737c43050b5546e: function (arg0, arg1, arg2) {
            const ret = Array.of(arg0, arg1, arg2);
            return ret;
        },
        __wbg_performance_3fcf6e32a7e1ed0a: function (arg0) {
            const ret = arg0.performance;
            return ret;
        },
        __wbg_postMessage_0162be6e48cf631e: function () {
            return handleError(function (arg0, arg1) {
                arg0.postMessage(arg1);
            }, arguments);
        },
        __wbg_process_44c7a14e11e9f69e: function (arg0) {
            const ret = arg0.process;
            return ret;
        },
        __wbg_prototypesetcall_d721637c7ca66eb8: function (arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_queueMicrotask_1c9b3800e321a967: function (arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_311744e534a929a3: function (arg0) {
            queueMicrotask(arg0);
        },
        __wbg_randomFillSync_6c25eac9869eb53c: function () {
            return handleError(function (arg0, arg1) {
                arg0.randomFillSync(arg1);
            }, arguments);
        },
        __wbg_require_b4edbdcf3e2a1ef0: function () {
            return handleError(function () {
                const ret = module.require;
                return ret;
            }, arguments);
        },
        __wbg_resolve_d82363d90af6928a: function (arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_setTimeout_f757f00851f76c42: function (arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        },
        __wbg_set_body_97c25d1c0051cb04: function (arg0, arg1) {
            arg0.body = arg1;
        },
        __wbg_set_cache_47f0e68e0309bb63: function (arg0, arg1) {
            arg0.cache = __wbindgen_enum_RequestCache[arg1];
        },
        __wbg_set_credentials_8dece1804391d22f: function (arg0, arg1) {
            arg0.credentials = __wbindgen_enum_RequestCredentials[arg1];
        },
        __wbg_set_headers_6751c09a8e579ff7: function (arg0, arg1) {
            arg0.headers = arg1;
        },
        __wbg_set_method_1120482abe0934aa: function (arg0, arg1, arg2) {
            arg0.method = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_mode_e41f820af904cdaa: function (arg0, arg1) {
            arg0.mode = __wbindgen_enum_RequestMode[arg1];
        },
        __wbg_set_onmessage_d05709471e546dca: function (arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_signal_4a69430cb12800f3: function (arg0, arg1) {
            arg0.signal = arg1;
        },
        __wbg_signal_4d9d567be73ea52c: function (arg0) {
            const ret = arg0.signal;
            return ret;
        },
        __wbg_stack_3b0d974bbf31e44f: function (arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_THIS_2fee5048bcca5938: function () {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_ce44e66a4935da8c: function () {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_44f6e0cb5e67cdad: function () {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_168f178805d978fe: function () {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_status_0053aa6239760447: function (arg0) {
            const ret = arg0.status;
            return ret;
        },
        __wbg_stringify_747a843de2eb6359: function () {
            return handleError(function (arg0) {
                const ret = JSON.stringify(arg0);
                return ret;
            }, arguments);
        },
        __wbg_subarray_b0e8ac4ed313fea8: function (arg0, arg1, arg2) {
            const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_text_68ea00f7126f2706: function () {
            return handleError(function (arg0) {
                const ret = arg0.text();
                return ret;
            }, arguments);
        },
        __wbg_then_05edfc8a4fea5106: function (arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_591b6b3a75ee817a: function (arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_then_c768c7c3e60c20ef: function (arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_timeOrigin_f3d5cb4f4a06c2b7: function (arg0) {
            const ret = arg0.timeOrigin;
            return ret;
        },
        __wbg_url_0e0eeabf01fb5519: function (arg0, arg1) {
            const ret = arg1.url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_2f34afb824ffcd9a: function (arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_value_49f783bb59765962: function (arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_versions_276b2795b1c6a219: function (arg0) {
            const ret = arg0.versions;
            return ret;
        },
        __wbg_waitAsync_134f0b4abc50f1f2: function (arg0, arg1, arg2) {
            const ret = Atomics.waitAsync(arg0, arg1 >>> 0, arg2);
            return ret;
        },
        __wbg_waitAsync_485a8d512901fd53: function () {
            const ret = Atomics.waitAsync;
            return ret;
        },
        __wbg_warn_88c4a5bd9a322000: function (arg0, arg1, arg2, arg3) {
            console.warn(arg0, arg1, arg2, arg3);
        },
        __wbg_wasmrpcserver_new: function (arg0) {
            const ret = WasmRpcServer.__wrap(arg0);
            return ret;
        },
        __wbindgen_cast_0000000000000001: function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 1221, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__hfad6e35b85c7033f);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 4724, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h60a57e826f0fa70e);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 4741, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h1506b1d2a44b3513);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 4360, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h204019b3324ffeea);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function (arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000006: function (arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000007: function (arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function () {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
        __wbindgen_link_ec60efd85dcca315: function (arg0) {
            const val = `onmessage = function (ev) {
                let [ia, index, value] = ev.data;
                ia = new Int32Array(ia.buffer);
                let result = Atomics.wait(ia, index, value);
                postMessage(result);
            };
            `;
            const ret = typeof URL.createObjectURL === 'undefined' ? "data:application/javascript," + encodeURIComponent(val) : URL.createObjectURL(new Blob([val], { type: "text/javascript" }));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        memory: memory || new WebAssembly.Memory({ initial: 29, maximum: 16384, shared: true }),
    };
    return {
        __proto__: null,
        "./psy_prover_bg.js": import0,
    };
}
function wasm_bindgen__convert__closures_____invoke__h204019b3324ffeea(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__h204019b3324ffeea(arg0, arg1);
}
function wasm_bindgen__convert__closures_____invoke__hfad6e35b85c7033f(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__hfad6e35b85c7033f(arg0, arg1, arg2);
}
function wasm_bindgen__convert__closures_____invoke__h1506b1d2a44b3513(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h1506b1d2a44b3513(arg0, arg1, arg2);
}
function wasm_bindgen__convert__closures_____invoke__h60a57e826f0fa70e(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h60a57e826f0fa70e(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}
function wasm_bindgen__convert__closures_____invoke__h18544ad86c9831de(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h18544ad86c9831de(arg0, arg1, arg2, arg3);
}
const __wbindgen_enum_RequestCache = ["default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached"];
const __wbindgen_enum_RequestCredentials = ["omit", "same-origin", "include"];
const __wbindgen_enum_RequestMode = ["same-origin", "no-cors", "cors", "navigate"];
const WasmConstantsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => { }, unregister: () => { } }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmconstants_free(ptr, 1));
const WasmPsyConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => { }, unregister: () => { } }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpsyconfig_free(ptr, 1));
const WasmPsyConfigBuilderFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => { }, unregister: () => { } }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpsyconfigbuilder_free(ptr, 1));
const WasmRpcServerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => { }, unregister: () => { } }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmrpcserver_free(ptr, 1));
function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}
const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => { }, unregister: () => { } }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));
function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        }
        else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        }
        else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for (let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    }
    else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        }
        catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}
function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
}
function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}
let cachedBigUint64ArrayMemory0 = null;
function getBigUint64ArrayMemory0() {
    if (cachedBigUint64ArrayMemory0 === null || cachedBigUint64ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedBigUint64ArrayMemory0 = new BigUint64Array(wasm.memory.buffer);
    }
    return cachedBigUint64ArrayMemory0;
}
let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer !== wasm.memory.buffer) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}
function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}
let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}
function handleError(f, args) {
    try {
        return f.apply(this, args);
    }
    catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}
function isLikeNone(x) {
    return x === undefined || x === null;
}
function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        }
        finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}
function passArray64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getBigUint64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }
    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;
    const mem = getUint8ArrayMemory0();
    let offset = 0;
    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F)
            break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);
        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }
    WASM_VECTOR_LEN = offset;
    return ptr;
}
function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}
let cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : undefined);
if (cachedTextDecoder)
    cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().slice(ptr, ptr + len));
}
const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder() : undefined);
if (cachedTextEncoder) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}
let WASM_VECTOR_LEN = 0;
let wasm;
function __wbg_finalize_init(instance, module, thread_stack_size) {
    wasm = instance.exports;
    cachedBigUint64ArrayMemory0 = null;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    if (typeof thread_stack_size !== 'undefined' && (typeof thread_stack_size !== 'number' || thread_stack_size === 0 || thread_stack_size % 65536 !== 0)) {
        throw new Error('invalid stack size');
    }
    wasm.__wbindgen_start(thread_stack_size);
    return wasm;
}
function initSync(module, memory) {
    if (wasm !== undefined)
        return wasm;
    let thread_stack_size;
    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({ module, memory, thread_stack_size } = module);
        }
        else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead');
        }
    }
    const imports = __wbg_get_imports(memory);
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module, thread_stack_size);
}

export { WasmConstants, WasmPsyConfig, WasmPsyConfigBuilder, WasmRpcServer, initSync, init_logging, main };
