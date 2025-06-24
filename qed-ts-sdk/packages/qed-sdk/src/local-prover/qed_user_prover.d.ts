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
