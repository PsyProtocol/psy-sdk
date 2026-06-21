import { Hash256, SCNumberLike } from "../core";
interface IPreparedPsySigAction {
    network_magic: bigint;
    user: bigint;
    sig_action: bigint;
    nonce: bigint;
    action_arguments: bigint[];
}
interface IPsySigAction {
    network_magic: string;
    user: string;
    sig_action: string;
    nonce: string;
    action_arguments: string[];
}
interface IPsyClaimDepositRequest {
    network_magic: string;
    user: number;
    transaction_id: Hash256;
    amount: SCNumberLike;
    deposit_fee: number;
}
interface IPsyTransferRequest {
    network_magic: string;
    user: number;
    nonce: SCNumberLike;
    recipient: number;
    amount: SCNumberLike;
}
interface IPsyWithdrawalRequest {
    network_magic: string;
    user: number;
    nonce: SCNumberLike;
    address: string;
    amount: SCNumberLike;
    withdrawal_fee: number;
}
export type { IPsySigAction, IPsyClaimDepositRequest, IPsyTransferRequest, IPsyWithdrawalRequest, IPreparedPsySigAction, };
//# sourceMappingURL=types.d.ts.map