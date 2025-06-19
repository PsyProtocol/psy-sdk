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
