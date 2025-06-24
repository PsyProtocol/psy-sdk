/* tslint:disable */
/* eslint-disable */
export function init_logging(): void;
export function main(): void;
export class WasmRpcServer {
    free(): void;
    constructor(rpc_config_json: string);
    start_session(): Promise<string>;
    prove_contract_call_json(contract_call_json: string): Promise<string>;
    prove_contract_calls_json(contract_calls_json: string): Promise<string>;
    sign_and_submit(): Promise<string>;
    register_user(private_key_str: string): Promise<string>;
    add_user(private_key_str: string): Promise<string>;
    switch_user(pk_hash_str: string): Promise<void>;
    get_zk_public_key_json(private_key_str: string): string;
    get_random_keypair_json(): string;
    deploy_contract_json(circuit_defs_json: string): Promise<string>;
    get_deploy_contract_cmd_json(circuit_defs_json: string): string;
    get_sighash(network_magic: bigint): string;
    get_zk_signature_json(sighash_str: string): string;
    get_end_cap_proof_json(signature_proof_json: string): string;
    get_user_ec_input_json(): Promise<string>;
    ping(message: string): string;
    get_result(id_str: string): Uint8Array;
}
