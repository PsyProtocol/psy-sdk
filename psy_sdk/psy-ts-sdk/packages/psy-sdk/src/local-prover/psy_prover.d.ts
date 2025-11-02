/* tslint:disable */
/* eslint-disable */
export function main(): void;
export function init_logging(): void;
export class WasmConstants {
  private constructor();
  free(): void;
  /**
   * Get all constants as a JSON string for easier JS consumption
   */
  static getAllConstants(): string;
  static readonly global_user_tree_height: number;
  static readonly coordinator_user_tree_height: number;
  static readonly realm_user_tree_height: number;
  static readonly group_realm_height: number;
  static readonly users_per_realm: bigint;
  static readonly native_currency_decimal: number;
  static readonly native_currency: string;
  static readonly native_currency_name: string;
  static readonly register_user_fee: bigint;
  static readonly deploy_contract_fee: bigint;
  static readonly guta_fee: bigint;
  static readonly current_network: string;
  static readonly config_path: string;
  static readonly coordinator_rpc_url: string;
  static readonly realm_rpc_urls: string[];
}
export class WasmPsyConfig {
  free(): void;
  constructor(json: string);
  useNetwork(network_name: string): void;
  getCurrentNetwork(): string;
  /**
   * Create using builder pattern (for more complex configurations)
   */
  static builder(): WasmPsyConfigBuilder;
  getNetworkJson(network_name: string): string;
  listNetworks(): string[];
  currentNetworkName(): string;
}
/**
 * WASM Builder for flexible configuration in browser/JS environments
 */
export class WasmPsyConfigBuilder {
  free(): void;
  constructor();
  /**
   * Set configuration from JSON string
   */
  json(json: string): WasmPsyConfigBuilder;
  /**
   * Set initial network to use
   */
  network(network: string): WasmPsyConfigBuilder;
  /**
   * Build the configuration
   */
  build(): WasmPsyConfig;
}
export class WasmRpcServer {
  free(): void;
  constructor(rpc_config_json: string);
  exec_contract_call_json(pk_hash: string, contract_calls_json: string): Promise<string>;
  exec_contract_call_with_sign_data_json(_pk_hash: string, _contract_calls_json: string, _sign_data?: string | null): Promise<string>;
  get_claim_rewards_call_args_json(job_infos_json: string): Promise<string>;
  claim_rewards_json(pk_hash: string, job_infos_json: string): Promise<string>;
  start_session(pk_hash: string): Promise<string>;
  prove_contract_call_json(pk_hash: string, contract_call_json: string): Promise<string>;
  prove_contract_calls_json(pk_hash: string, contract_calls_json: string): Promise<string>;
  sign_and_submit(pk_hash: string): Promise<string>;
  sign_and_submit_with_sign_data(pk_hash: string, sign_data?: string | null): Promise<string>;
  register_user(private_key_str: string): Promise<string>;
  register_user_with_type(private_key: string, sign_type: string, fingerprint?: string | null): Promise<string>;
  add_user(private_key_str: string): Promise<string>;
  add_user_with_type(private_key_str: string, sign_type: string, fingerprint?: string | null): Promise<string>;
  get_zk_public_key_json(private_key_str: string): Promise<string>;
  get_random_keypair_json(): Promise<string>;
  deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
  get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
  ping(message: string): string;
  get_result(id_str: string): Uint8Array;
}
