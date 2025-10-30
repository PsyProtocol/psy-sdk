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
