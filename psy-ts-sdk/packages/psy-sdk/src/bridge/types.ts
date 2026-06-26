import type {
    BridgeDepositBatchGroth16Proof,
    BridgeDepositBatchWitnessInput,
    BridgeWithdrawalGroth16Proof,
    BridgeWithdrawalWitnessInput,
} from "../local-prover-rpc/types";

export type U32x8 = [number, number, number, number, number, number, number, number];

export interface ServicesApiResponse<T> {
    success: boolean;
    data?: T;
    error?: string;
    timestamp?: string;
}

export interface DepositClaimProofQuery {
    depositor: string;
    nonce: number | bigint | string;
    sourceChainId: number | bigint | string;
    depositIndex?: number | bigint | string;
}

export interface DepositClaimProofResult {
    found: boolean;
    deposit_index?: number;
    leaf_hash?: string;
    leaf_index?: number;
    deposit_root?: string;
    siblings?: string[];
}

export interface WithdrawalClaimProofQuery {
    recipient: string;
    tokenAddress: string;
    amount: string;
    nonce: number | bigint | string;
    destinationChainId: number | bigint | string;
    senderUserId?: number | bigint | string;
}

export interface WithdrawalClaimProofResult {
    found: boolean;
    leaf_index?: number;
    siblings?: string[];
    withdrawal_root?: string;
    withdrawal?: {
        recipient: string;
        token_address: string;
        amount: string;
        nonce: string;
        destination_chain_id: number;
        sender_user_id?: number;
        withdrawal_index?: number;
    };
}

export interface JsonRpcResponse<T> {
    jsonrpc: "2.0";
    id: string | number | null;
    result?: T;
    error?: {
        code: number;
        message: string;
        data?: unknown;
    };
}

export interface BridgeProveWithdrawalRequest {
    witnessInput: BridgeWithdrawalWitnessInput;
}

export interface BridgeProveDepositBatchRequest {
    witnessInput: BridgeDepositBatchWitnessInput;
}

export type {
    BridgeDepositBatchGroth16Proof,
    BridgeDepositBatchWitnessInput,
    BridgeWithdrawalGroth16Proof,
    BridgeWithdrawalWitnessInput,
};
