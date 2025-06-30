/* tslint:disable */
/* eslint-disable */
export function init_logging(): void;
export function main(): void;
export class WasmRpcServer {
  free(): void;
  constructor(rpc_config_json: string);
  exec_contract_call_json(pk_hash: string, contract_calls_json: string): Promise<string>;
  start_session(pk_hash: string): Promise<string>;
  prove_contract_call_json(pk_hash: string, contract_call_json: string): Promise<string>;
  prove_contract_calls_json(pk_hash: string, contract_calls_json: string): Promise<string>;
  sign_and_submit(pk_hash: string): Promise<string>;
  register_user(private_key_str: string): Promise<string>;
  add_user(private_key_str: string): Promise<string>;
  get_zk_public_key_json(private_key_str: string): Promise<string>;
  get_random_keypair_json(): Promise<string>;
  deploy_contract_json(deployer: string, circuit_defs_json: string): Promise<string>;
  get_deploy_contract_cmd_json(deployer: string, circuit_defs_json: string): Promise<string>;
  ping(message: string): Promise<string>;
  get_result(id_str: string): Promise<Uint8Array>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmrpcserver_free: (a: number, b: number) => void;
  readonly wasmrpcserver_new: (a: number, b: number) => [number, number, number];
  readonly wasmrpcserver_exec_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_start_session: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_prove_contract_call_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_prove_contract_calls_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_sign_and_submit: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_register_user: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_add_user: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_get_zk_public_key_json: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_get_random_keypair_json: (a: number) => any;
  readonly wasmrpcserver_deploy_contract_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_get_deploy_contract_cmd_json: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmrpcserver_ping: (a: number, b: number, c: number) => any;
  readonly wasmrpcserver_get_result: (a: number, b: number, c: number) => any;
  readonly init_logging: () => void;
  readonly main: () => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hb74dce0a84f3361e: (a: number, b: number) => void;
  readonly closure1284_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure1473_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
