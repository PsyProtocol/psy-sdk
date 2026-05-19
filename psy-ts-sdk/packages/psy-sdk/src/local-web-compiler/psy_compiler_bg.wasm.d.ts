/* tslint:disable */
/* eslint-disable */
export const call_contract: (a: bigint, b: bigint, c: number, d: number, e: number, f: number) => [number, number];
export const compile_project: (a: number, b: number) => [number, number];
export const compile_sdk_key_project: (a: number, b: number) => [number, number];
export const compile_sdk_key_source: (a: number, b: number) => [number, number];
export const compile_source: (a: number, b: number) => [number, number];
export const create_account: (a: number, b: number) => [number, number];
export const deploy_contract: (a: bigint) => [number, number];
export const get_accounts: () => [number, number];
export const get_contract_abi: (a: bigint) => [number, number];
export const get_contracts: () => [number, number];
export const get_transaction_log: () => [number, number];
export const read_contract_state: (a: bigint, b: bigint) => [number, number];
export const read_imt_state: (a: number, b: number) => [number, number];
export const init_chain: () => void;
export const reset_chain: () => void;
export const init_psy_ide: () => void;
export const memory: WebAssembly.Memory;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __wbindgen_thread_destroy: (a?: number, b?: number, c?: number) => void;
export const __wbindgen_start: (a: number) => void;
