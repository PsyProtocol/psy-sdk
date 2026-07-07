/* tslint:disable */
/* eslint-disable */

export function call_contract(caller_id: bigint, contract_id: bigint, method_name: string, args_json: string): string;

export function compile_dargo_project(project_json: string): string;

export function compile_project(files_json: string): string;

export function compile_source(source: string): string;

export function create_account(name: string): string;

export function deploy_contract(deployer_id: bigint): string;

export function get_accounts(): string;

export function get_contract_abi(contract_id: bigint): string;

export function get_contracts(): string;

export function get_transaction_log(): string;

export function init_chain(): void;

export function init_logging(): void;

export function init_psy_ide(): void;

export function interpret_project(files_json: string, request_json: string): string;

export function interpret_source(source: string, request_json: string): string;

export function main(): void;

export function read_contract_state(contract_id: bigint, user_id: bigint): string;

export function read_imt_state(contract_id: number, user_id: number): string;

export function reset_chain(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly call_contract: (a: bigint, b: bigint, c: number, d: number, e: number, f: number) => [number, number];
    readonly compile_dargo_project: (a: number, b: number) => [number, number];
    readonly compile_project: (a: number, b: number) => [number, number];
    readonly compile_source: (a: number, b: number) => [number, number];
    readonly create_account: (a: number, b: number) => [number, number];
    readonly deploy_contract: (a: bigint) => [number, number];
    readonly get_accounts: () => [number, number];
    readonly get_contract_abi: (a: bigint) => [number, number];
    readonly get_contracts: () => [number, number];
    readonly get_transaction_log: () => [number, number];
    readonly interpret_project: (a: number, b: number, c: number, d: number) => [number, number];
    readonly interpret_source: (a: number, b: number, c: number, d: number) => [number, number];
    readonly read_contract_state: (a: bigint, b: bigint) => [number, number];
    readonly read_imt_state: (a: number, b: number) => [number, number];
    readonly init_logging: () => void;
    readonly init_psy_ide: () => void;
    readonly main: () => void;
    readonly init_chain: () => void;
    readonly reset_chain: () => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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
