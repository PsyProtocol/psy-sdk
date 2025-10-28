import { Hash256, SCNumberLike } from "../core";

interface IPreparedQedSigAction {
    network_magic: bigint;
    user: bigint;
    sig_action: bigint;
    nonce: bigint;
    action_arguments: bigint[];
}
interface IQedSigAction {
    network_magic: string;
    user: string;
    sig_action: string;
    nonce: string;
    action_arguments: string[];
}

interface IQedClaimDepositRequest {
    network_magic: string;
    user: number;
    transaction_id: Hash256;
    amount: SCNumberLike;
    deposit_fee: number;
}

interface IQedTransferRequest {
    network_magic: string;
    user: number;
    nonce: SCNumberLike;
    recipient: number;
    amount: SCNumberLike;
}

interface IQedWithdrawalRequest {
    network_magic: string;
    user: number;
    nonce: SCNumberLike;
    l1_address: string;
    amount: SCNumberLike;
    withdrawal_fee: number;
}

export type {
    IQedSigAction,
    IQedClaimDepositRequest,
    IQedTransferRequest,
    IQedWithdrawalRequest,
    IPreparedQedSigAction,
};
