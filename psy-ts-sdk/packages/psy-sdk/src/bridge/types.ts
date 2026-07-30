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
    /** Envio global deposit locator (bridge list/display identity). */
    deposit_index?: number;
    /** Per-source-chain tree leaf index; equals depositProofRaw.deposit_index. */
    chain_local_deposit_index?: number;
    source_chain_index?: number;
    proved_deposit_count?: number;
    snapshot_deposit_count?: number;
    proved_count?: number;
    tree_count?: number;
    leaf_hash?: string;
    leaf_index?: number;
    deposit_root?: string;
    siblings?: string[];
    deposit?: {
        shield_address: string;
        token_address: string;
        l2_token_contract_id: string;
        amount: string;
        note_commitment: string;
        source_chain_id: number;
        created_at?: string;
    };
}

export interface WithdrawalClaimProofQuery {
    recipient: string;
    tokenAddress: string;
    amount: string;
    nonce: string;
    destinationChainIndex: number | string;
    senderUserId?: number | string;
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
        destination_chain_index: number;
        sender_user_id?: number;
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
