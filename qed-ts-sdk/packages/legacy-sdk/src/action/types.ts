import { Hash256, SCNumberLike } from "../rpc/baseTypes";

interface IPreparedCitySigAction {
    network_magic: bigint;
    user: bigint;
    sig_action: bigint;
    nonce: bigint;
    action_arguments: bigint[];
}
interface ICitySigAction {
    network_magic: string;
    user: string;
    sig_action: string;
    nonce: string;
    action_arguments: string[];
}

interface ICityClaimDepositRequest {
    network_magic: string;
    user: number;
    transaction_id: Hash256;
    amount: SCNumberLike;
    deposit_fee: number;
}

interface ICityTransferRequest {
    network_magic: string;
    user: number;
    nonce: SCNumberLike;
    recipient: number;
    amount: SCNumberLike;
}

interface ICityWithdrawalRequest {
    network_magic: string;
    user: number;
    nonce: SCNumberLike;
    l1_address: string;
    amount: SCNumberLike;
    withdrawal_fee: number;
}

export type {
    ICitySigAction,
    ICityClaimDepositRequest,
    ICityTransferRequest,
    ICityWithdrawalRequest,
    IPreparedCitySigAction,
};
