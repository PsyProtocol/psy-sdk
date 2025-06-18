/* tslint:disable */
/* eslint-disable */
export function wasm_main(): void;
export function init_logging(): void;
export class WasmRpcServer {
    free(): void;
    constructor(rpc_config_json: string);
    start_session(): string;
    prove_contract_call_json(contract_call_json: string): string;
    prove_contract_calls_json(contract_calls_json: string): string;
    sign_and_submit(): string;
    register_user(private_key_str: string): string;
    add_user(private_key_str: string): string;
    switch_user(pk_hash_str: string): void;
    get_zk_public_key_json(private_key_str: string): string;
    get_random_keypair_json(): string;
    deploy_contract_json(circuit_defs_json: string): string;
    get_deploy_contract_cmd_json(circuit_defs_json: string): string;
    get_sighash(network_magic: bigint): string;
    get_zk_signature_json(sighash_str: string): string;
    get_end_cap_proof_json(signature_proof_json: string): string;
    get_user_ec_input_json(): string;
    ping(message: string): string;
    get_result(id_str: string): Uint8Array;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmrpcserver_free: (a: number, b: number) => void;
    readonly wasmrpcserver_new: (a: number, b: number) => [number, number, number];
    readonly wasmrpcserver_start_session: (a: number) => [number, number, number, number];
    readonly wasmrpcserver_prove_contract_call_json: (
        a: number,
        b: number,
        c: number
    ) => [number, number, number, number];
    readonly wasmrpcserver_prove_contract_calls_json: (
        a: number,
        b: number,
        c: number
    ) => [number, number, number, number];
    readonly wasmrpcserver_sign_and_submit: (a: number) => [number, number, number, number];
    readonly wasmrpcserver_register_user: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_add_user: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_switch_user: (a: number, b: number, c: number) => [number, number];
    readonly wasmrpcserver_get_zk_public_key_json: (
        a: number,
        b: number,
        c: number
    ) => [number, number, number, number];
    readonly wasmrpcserver_get_random_keypair_json: (a: number) => [number, number, number, number];
    readonly wasmrpcserver_deploy_contract_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_get_deploy_contract_cmd_json: (
        a: number,
        b: number,
        c: number
    ) => [number, number, number, number];
    readonly wasmrpcserver_get_sighash: (a: number, b: bigint) => [number, number, number, number];
    readonly wasmrpcserver_get_zk_signature_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_get_end_cap_proof_json: (
        a: number,
        b: number,
        c: number
    ) => [number, number, number, number];
    readonly wasmrpcserver_get_user_ec_input_json: (a: number) => [number, number, number, number];
    readonly wasmrpcserver_ping: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrpcserver_get_result: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasm_main: () => void;
    readonly init_logging: () => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_export_2: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export_6: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h748cf0f517aabec2: (
        a: number,
        b: number
    ) => void;
    readonly closure1468_externref_shim: (a: number, b: number, c: any) => void;
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
export default function __wbg_init(
    module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>
): Promise<InitOutput>;
