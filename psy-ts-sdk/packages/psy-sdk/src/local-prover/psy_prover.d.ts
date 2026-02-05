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
    add_user(private_key_str: string, sign_type: string): Promise<string>;
    deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
    exec_contract_call_json(pk_hash: string, call_data_json: string): Promise<string>;
    get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
    get_random_keypair_json(): Promise<string>;
    get_result(id_str: string): Uint8Array;
    get_zk_public_key_json(private_key_str: string): Promise<string>;
    constructor(rpc_config_json: string);
    ping(message: string): string;
    prove_contract_call_json(pk_hash: string, contract_call_json: string): Promise<string>;
    prove_contract_calls_json(pk_hash: string, contract_calls_json: string): Promise<string>;
    register_user(private_key_str: string, sign_type: string): Promise<string>;
    sign_and_submit(pk_hash: string, sign_data?: string | null): Promise<string>;
    start_session(pk_hash: string): Promise<string>;
}

export function init_logging(): void;

export function main(): void;
