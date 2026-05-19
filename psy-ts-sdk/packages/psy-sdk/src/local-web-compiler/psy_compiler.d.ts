/* tslint:disable */
/* eslint-disable */

/**
 * Call a contract method. Returns JSON with execution result.
 */
export function call_contract(caller_id: bigint, contract_id: bigint, method_name: string, args_json: string): string;

/**
 * Compile a multi-file PSY project. `files_json` is a JSON array of
 * `[module_path_parts[], source_text]` pairs.
 */
export function compile_project(files_json: string): string;

/**
 * Compile a multi-file PSY project as an SDK key. Returns JSON.
 *
 * `files_json` is a JSON array of `[module_path_parts[], source_text]` pairs.
 * The result includes both DPN bytecode (JSON) and Dapen bytecode (hex CBOR).
 */
export function compile_sdk_key_project(files_json: string): string;

/**
 * Compile a single-file PSY source as an SDK key. Returns JSON.
 *
 * The result includes both DPN bytecode (JSON) and Dapen bytecode (hex CBOR).
 */
export function compile_sdk_key_source(source: string): string;

/**
 * Compile a single-file PSY source. Returns JSON string.
 */
export function compile_source(source: string): string;

/**
 * Create a user account. Returns JSON.
 */
export function create_account(name: string): string;

/**
 * Deploy the last compiled contract. Returns JSON.
 */
export function deploy_contract(deployer_id: bigint): string;

/**
 * Get all accounts. Returns JSON array.
 */
export function get_accounts(): string;

/**
 * Get the ABI for a deployed contract. Returns JSON.
 */
export function get_contract_abi(contract_id: bigint): string;

/**
 * Get all deployed contracts. Returns JSON array.
 */
export function get_contracts(): string;

/**
 * Get the full transaction log. Returns JSON array.
 */
export function get_transaction_log(): string;

/**
 * Initialize the in-memory chain.
 */
export function init_chain(): void;

export function init_psy_ide(): void;

/**
 * Read contract state for a specific user. Returns JSON array of slot values.
 */
export function read_contract_state(contract_id: bigint, user_id: bigint): string;

/**
 * Read IMT (Indexed Merkle Tree) key-value entries for a specific
 * user/contract. Returns JSON array of `{key: [u64;4], value: [u64;4]}`
 * objects.
 */
export function read_imt_state(contract_id: number, user_id: number): string;

/**
 * Reset the chain (clear all state).
 */
export function reset_chain(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly call_contract: (a: bigint, b: bigint, c: number, d: number, e: number, f: number) => [number, number];
  readonly compile_project: (a: number, b: number) => [number, number];
  readonly compile_sdk_key_project: (a: number, b: number) => [number, number];
  readonly compile_sdk_key_source: (a: number, b: number) => [number, number];
  readonly compile_source: (a: number, b: number) => [number, number];
  readonly create_account: (a: number, b: number) => [number, number];
  readonly deploy_contract: (a: bigint) => [number, number];
  readonly get_accounts: () => [number, number];
  readonly get_contract_abi: (a: bigint) => [number, number];
  readonly get_contracts: () => [number, number];
  readonly get_transaction_log: () => [number, number];
  readonly read_contract_state: (a: bigint, b: bigint) => [number, number];
  readonly read_imt_state: (a: number, b: number) => [number, number];
  readonly init_chain: () => void;
  readonly reset_chain: () => void;
  readonly init_psy_ide: () => void;
  readonly memory: WebAssembly.Memory;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_externrefs: WebAssembly.Table;
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
