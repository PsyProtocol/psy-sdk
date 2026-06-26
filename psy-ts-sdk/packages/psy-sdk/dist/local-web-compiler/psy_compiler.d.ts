/**
 * @param {bigint} caller_id
 * @param {bigint} contract_id
 * @param {string} method_name
 * @param {string} args_json
 * @returns {string}
 */
export function call_contract(caller_id: bigint, contract_id: bigint, method_name: string, args_json: string): string;
/**
 * @param {string} project_json
 * @returns {string}
 */
export function compile_dargo_project(project_json: string): string;
/**
 * @param {string} files_json
 * @returns {string}
 */
export function compile_project(files_json: string): string;
/**
 * @param {string} source
 * @returns {string}
 */
export function compile_source(source: string): string;
/**
 * @param {string} name
 * @returns {string}
 */
export function create_account(name: string): string;
/**
 * @param {bigint} deployer_id
 * @returns {string}
 */
export function deploy_contract(deployer_id: bigint): string;
/**
 * @returns {string}
 */
export function get_accounts(): string;
/**
 * @param {bigint} contract_id
 * @returns {string}
 */
export function get_contract_abi(contract_id: bigint): string;
/**
 * @returns {string}
 */
export function get_contracts(): string;
/**
 * @returns {string}
 */
export function get_transaction_log(): string;
export function init_chain(): void;
export function init_logging(): void;
export function init_psy_ide(): void;
/**
 * @param {string} files_json
 * @param {string} request_json
 * @returns {string}
 */
export function interpret_project(files_json: string, request_json: string): string;
/**
 * @param {string} source
 * @param {string} request_json
 * @returns {string}
 */
export function interpret_source(source: string, request_json: string): string;
export function main(): void;
/**
 * @param {bigint} contract_id
 * @param {bigint} user_id
 * @returns {string}
 */
export function read_contract_state(contract_id: bigint, user_id: bigint): string;
/**
 * @param {number} contract_id
 * @param {number} user_id
 * @returns {string}
 */
export function read_imt_state(contract_id: number, user_id: number): string;
export function reset_chain(): void;
export function initSync(module: any): any;
declare function __wbg_init(module_or_path: any): Promise<any>;
export { __wbg_init as default };
//# sourceMappingURL=psy_compiler.d.ts.map