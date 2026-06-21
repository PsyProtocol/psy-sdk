import type { DPNFunctionCircuitDefinition } from "../local-prover-rpc/types";
export interface PsyCompileResult {
    success: boolean;
    error?: string;
    error_offset?: number;
    entry_path?: string;
    compile_results?: DPNFunctionCircuitDefinition[];
    abi?: any;
}
export interface PsyInterpretResult {
    success: boolean;
    error?: string;
    error_offset?: number;
    entry_path?: string;
    execution_result?: any;
    compile_results?: DPNFunctionCircuitDefinition[];
    abi?: any;
}
export interface PsyProjectInput {
    entry: string[];
    method_names?: string[];
    files: [string[], string][];
}
export interface PsyInterpretRequest {
    method_name?: string;
    inputs?: (number | bigint)[];
    execution_context?: {
        user_id?: number | bigint;
        contract_id?: number | bigint;
        caller_contract_id?: number | bigint;
        checkpoint_id?: number | bigint;
        nonce?: number | bigint;
        user_public_key_hash?: [number | bigint, number | bigint, number | bigint, number | bigint];
    };
    initial_state?: {
        slots?: Array<{
            user_id: number | bigint;
            contract_id: number | bigint;
            slot_index: number | bigint;
            value: number | bigint;
        }>;
        hashes?: Array<{
            user_id: number | bigint;
            contract_id: number | bigint;
            slot_index: number | bigint;
            value: [number | bigint, number | bigint, number | bigint, number | bigint];
        }>;
        deployers?: Array<{
            contract_id: number | bigint;
            deployer: [number | bigint, number | bigint, number | bigint, number | bigint];
        }>;
        checkpoint_stats?: Array<{
            checkpoint_id: number | bigint;
            values: Array<number | bigint>;
        }>;
        contract_leaves?: Array<{
            contract_id: number | bigint;
            values: Array<number | bigint>;
        }>;
        checkpoint_global_state_roots?: Array<{
            checkpoint_id: number | bigint;
            values: Array<number | bigint>;
        }>;
        imt?: Array<{
            user_id: number | bigint;
            contract_id: number | bigint;
            key: [number | bigint, number | bigint, number | bigint, number | bigint];
            value: [number | bigint, number | bigint, number | bigint, number | bigint];
        }>;
    };
}
/**
 * Compile a single PSY source file in-browser.
 * @param source - PSY source code string
 */
export declare function compileSource(source: string): PsyCompileResult;
/**
 * Compile a multi-file PSY project in-browser.
 * @param input - Explicit project input.
 *   e.g. { entry: ["main"], method_names: ["main"], files: [[["main"], "mod foo;\nfn main() {}"], [["foo"], "pub fn run() {}"]] }
 */
export declare function compileProject(input: PsyProjectInput): PsyCompileResult;
export declare function interpretSource(source: string, request: PsyInterpretRequest): PsyInterpretResult;
export declare function interpretProject(input: PsyProjectInput, request: PsyInterpretRequest): PsyInterpretResult;
//# sourceMappingURL=compiler.d.ts.map