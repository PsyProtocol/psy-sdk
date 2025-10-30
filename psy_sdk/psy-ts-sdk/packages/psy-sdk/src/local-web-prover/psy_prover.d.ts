/* tslint:disable */
/* eslint-disable */
export function init_logging(): void;
export function main(): void;
export class WasmRpcServer {
  free(): void;
  get_result(id_str: string): Uint8Array;
  register_user(private_key_str: string): Promise<string>;
  start_session(pk_hash: string): Promise<string>;
  sign_and_submit(pk_hash: string): Promise<string>;
  add_user_with_type(private_key_str: string, sign_type: string, fingerprint?: string | null): Promise<string>;
  claim_rewards_json(pk_hash: string, job_infos_json: string): Promise<string>;
  deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
  get_zk_public_key_json(private_key_str: string): Promise<string>;
  exec_contract_call_json(pk_hash: string, contract_calls_json: string): Promise<string>;
  get_random_keypair_json(): Promise<string>;
  register_user_with_type(private_key: string, sign_type: string, fingerprint?: string | null): Promise<string>;
  prove_contract_call_json(pk_hash: string, contract_call_json: string): Promise<string>;
  prove_contract_calls_json(pk_hash: string, contract_calls_json: string): Promise<string>;
  get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): string;
  sign_and_submit_with_sign_data(pk_hash: string, sign_data?: string | null): Promise<string>;
  get_claim_rewards_call_args_json(job_infos_json: string): Promise<string>;
  exec_contract_call_with_sign_data_json(pk_hash: string, contract_calls_json: string, sign_data?: string | null): Promise<string>;
  constructor(rpc_config_json: string);
  ping(message: string): string;
  add_user(private_key_str: string): Promise<string>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly init_logging: () => void;
  readonly main: () => void;
  readonly __wbg_wasmrpcserver_free: (a: number, b: number) => void;
  readonly wasmrpcserver_add_user: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_add_user_with_type: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
  readonly wasmrpcserver_claim_rewards_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_deploy_contract_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_exec_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_exec_contract_call_with_sign_data_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
  readonly wasmrpcserver_get_claim_rewards_call_args_json: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_get_deploy_contract_cmd_json: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
  readonly wasmrpcserver_get_random_keypair_json: (a: number) => any;
  readonly wasmrpcserver_get_result: (a: number, b: number, c: number) => [number, number, number, number];
  readonly wasmrpcserver_get_zk_public_key_json: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_new: (a: number, b: number) => any;
  readonly wasmrpcserver_ping: (a: number, b: number, c: number) => [number, number, number, number];
  readonly wasmrpcserver_prove_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_prove_contract_calls_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_register_user: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_register_user_with_type: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
  readonly wasmrpcserver_sign_and_submit: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_sign_and_submit_with_sign_data: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_start_session: (a: number, b: number, c: number) => any;
  readonly memory: WebAssembly.Memory;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_3: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_7: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__hac89665212b2a743: (a: number, b: number) => void;
  readonly closure1877_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure2286_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
