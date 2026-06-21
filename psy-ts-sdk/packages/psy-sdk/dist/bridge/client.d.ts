import type { IHTTPClient } from "../http/types";
import type { BridgeDepositBatchGroth16Proof, BridgeDepositBatchWitnessInput, BridgeWithdrawalGroth16Proof, BridgeWithdrawalWitnessInput, DepositClaimProofQuery, DepositClaimProofResult, U32x8, WithdrawalClaimProofQuery, WithdrawalClaimProofResult } from "./types";
export declare function u32x8ToHex(words: readonly number[]): string;
export declare function hexToU32x8(hex: string): U32x8;
export declare class PoseidonBridgeClient {
    private readonly httpClient;
    constructor(httpClient?: IHTTPClient);
    getDepositClaimProof(servicesUrl: string, query: DepositClaimProofQuery): Promise<DepositClaimProofResult>;
    getWithdrawalClaimProof(servicesUrl: string, query: WithdrawalClaimProofQuery): Promise<WithdrawalClaimProofResult>;
    proveWithdrawalClaim(proveProxyUrl: string, witnessInput: BridgeWithdrawalWitnessInput): Promise<BridgeWithdrawalGroth16Proof>;
    proveDepositBatchAppend(proveProxyUrl: string, witnessInput: BridgeDepositBatchWitnessInput): Promise<BridgeDepositBatchGroth16Proof>;
}
//# sourceMappingURL=client.d.ts.map