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
  [Symbol.dispose](): void;
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

export class WasmPsyConfigBuilder {
  free(): void;
  [Symbol.dispose](): void;
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
  [Symbol.dispose](): void;
  constructor(rpc_config_json: string);
  exec_contract_call_json(pk_hash: string, call_data_json: string): Promise<string>;
  get_claim_rewards_call_args_json(job_infos_json: string): Promise<string>;
  claim_rewards_json(pk_hash: string, job_infos_json: string): Promise<string>;
  start_session(pk_hash: string): Promise<string>;
  prove_contract_call_json(pk_hash: string, contract_call_json: string): Promise<string>;
  prove_contract_calls_json(pk_hash: string, contract_calls_json: string): Promise<string>;
  sign_and_submit(pk_hash: string, sign_data?: string | null): Promise<string>;
  register_user(private_key_str: string, sign_type: string): Promise<string>;
  add_user(private_key_str: string, sign_type: string): Promise<string>;
  get_zk_public_key_json(private_key_str: string): Promise<string>;
  get_random_keypair_json(): Promise<string>;
  deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
  get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
  ping(message: string): string;
  get_result(id_str: string): Uint8Array;
}

export function init_logging(): void;

export function main(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly main: () => void;
  readonly init_logging: () => void;
  readonly __wbg_wasmpsyconfig_free: (a: number, b: number) => void;
  readonly wasmpsyconfig_new: (a: number, b: number) => [number, number, number];
  readonly wasmpsyconfig_useNetwork: (a: number, b: number, c: number) => [number, number];
  readonly wasmpsyconfig_getCurrentNetwork: (a: number) => [number, number, number, number];
  readonly wasmpsyconfig_builder: () => number;
  readonly wasmpsyconfig_getNetworkJson: (a: number, b: number, c: number) => [number, number, number, number];
  readonly wasmpsyconfig_listNetworks: (a: number) => [number, number];
  readonly wasmpsyconfig_currentNetworkName: (a: number) => [number, number];
  readonly __wbg_wasmpsyconfigbuilder_free: (a: number, b: number) => void;
  readonly wasmpsyconfigbuilder_json: (a: number, b: number, c: number) => number;
  readonly wasmpsyconfigbuilder_network: (a: number, b: number, c: number) => number;
  readonly wasmpsyconfigbuilder_build: (a: number) => [number, number, number];
  readonly __wbg_wasmconstants_free: (a: number, b: number) => void;
  readonly wasmconstants_global_user_tree_height: () => number;
  readonly wasmconstants_coordinator_user_tree_height: () => number;
  readonly wasmconstants_realm_user_tree_height: () => number;
  readonly wasmconstants_group_realm_height: () => number;
  readonly wasmconstants_users_per_realm: () => bigint;
  readonly wasmconstants_native_currency_decimal: () => number;
  readonly wasmconstants_native_currency: () => [number, number];
  readonly wasmconstants_native_currency_name: () => [number, number];
  readonly wasmconstants_deploy_contract_fee: () => bigint;
  readonly wasmconstants_guta_fee: () => bigint;
  readonly wasmconstants_current_network: () => [number, number];
  readonly wasmconstants_config_path: () => [number, number];
  readonly wasmconstants_coordinator_rpc_url: () => [number, number];
  readonly wasmconstants_realm_rpc_urls: () => [number, number];
  readonly wasmconstants_getAllConstants: () => [number, number, number, number];
  readonly __wbg_wasmrpcserver_free: (a: number, b: number) => void;
  readonly wasmrpcserver_new: (a: number, b: number) => any;
  readonly wasmrpcserver_exec_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_get_claim_rewards_call_args_json: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_claim_rewards_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_start_session: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_prove_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_prove_contract_calls_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_sign_and_submit: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_register_user: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_add_user: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_get_zk_public_key_json: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_get_random_keypair_json: (a: number) => any;
  readonly wasmrpcserver_deploy_contract_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_get_deploy_contract_cmd_json: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
  readonly wasmrpcserver_ping: (a: number, b: number, c: number) => [number, number, number, number];
  readonly wasmrpcserver_get_result: (a: number, b: number, c: number) => [number, number, number, number];
  readonly wasmpsyconfigbuilder_new: () => number;
  readonly wasmconstants_register_user_fee: () => bigint;
  readonly wasm_bindgen__convert__closures_____invoke__h2c2f1a4471ab6362: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h2db72ff56bc85308: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h9adef32405569d93: (a: number, b: number) => void;
  readonly wasm_bindgen__closure__destroy__h1c0ba36b9ad9b94b: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h01611f6b72c1e5a8: (a: number, b: number, c: any, d: any) => void;
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
