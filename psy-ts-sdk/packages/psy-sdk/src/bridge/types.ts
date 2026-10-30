import type {
    BridgeDepositBatchGroth16Proof,
    BridgeDepositBatchWitnessInput,
    BridgeWithdrawalBatchGroth16Proof,
    BridgeWithdrawalBatchWitnessInput,
} from "../local-prover-rpc/types";

export type U32x8 = [number, number, number, number, number, number, number, number];

export interface ServicesApiResponse<T> {
    success: boolean;
    data?: T;
    error?: string;
    timestamp?: string;
}

export interface DepositClaimProofQuery {
    /** Envio global deposit locator (bridge list/display identity). */
    depositIndex: number | bigint | string;
    /** Source chain the deposit was bridged through. */
    sourceChainIndex: number | bigint | string;
    /**
     * Chain-local tree leaf count at proof time. Preferred services input
     * (server-side aliases such as proved_deposit_count/tree_count are not
     * emitted by this client).
     */
    snapshotDepositCount: number | bigint | string;
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
    /** Leaf word 0. The L1 nullifier is the Poseidon hash of the full
     *  35-word leaf, not this sender and the nonce. */
    senderUserId: number | string;
    /** Leaf word 34. Required to recompute the withdrawal leaf hash. */
    tokenContractId: number | string;
}

export interface WithdrawalClaimProofResult {
    found: boolean;
    leaf_index?: number;
    /** Indexed Poseidon withdrawal leaf hash bytes32, when the service row
     *  carries it; the client rechecks it against the queried leaf hash. */
    leaf_hash?: string | null;
    siblings?: string[];
    withdrawal_root?: string;
    withdrawal?: {
        recipient: string | null;
        token_address: string | null;
        amount: string | null;
        nonce: string | null;
        destination_chain_index: number | null;
        /** Indexed withdrawal event emitter; required for the 35-word claim leaf. */
        token_contract_id: number | null;
        /** Required when withdrawal is present; null is valid for no-match or
         *  checkpoint-0 responses, not a matched withdrawal. */
        sender_user_id: number | null;
        /** Poseidon withdrawal leaf hash bytes32: eight u32 words, low half
         *  of each limb first, each word big-endian. The SDK passes the
         *  service value through and checks it against withdrawalLeafNullifier.
         *  Nullable on no-match only. */
        nullifier: string | null;
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

export interface BridgeProveWithdrawalBatchRequest {
    witnessInput: BridgeWithdrawalBatchWitnessInput;
}

export interface BridgeProveDepositBatchRequest {
    witnessInput: BridgeDepositBatchWitnessInput;
}

export type {
    BridgeDepositBatchGroth16Proof,
    BridgeDepositBatchWitnessInput,
    BridgeWithdrawalBatchGroth16Proof,
    BridgeWithdrawalBatchWitnessInput,
};
