import { IHashOut, hashNoPad } from "poseidon-goldilocks-lite";
import { SIG_ACTION_CLAIM_DEPOSIT_MAGIC, SIG_ACTION_TRANSFER_MAGIC, SIG_ACTION_WITHDRAW_MAGIC } from "./constants";
import { IPsyClaimDepositRequest, IPsySigAction, IPsyTransferRequest, IPsyWithdrawalRequest } from "./types";
import { SCNumberLike } from "../core";
import { getDecodedAddress } from "../utils/address";
import { readBigIntU48FromBytesLE, readBigIntU56FromBytesLE } from "../utils/data";
import { psyFelt, hash256ToHashOut224 } from "../utils/felt";

function getWithdrawalHashFromPublicKeyHash(
    value: bigint,
    publicKeyHash: Uint8Array,
    scriptTypeFlag: SCNumberLike
): IHashOut {
    const last48BitsWithFlag = readBigIntU48FromBytesLE(publicKeyHash, 14) | (BigInt(scriptTypeFlag) << BigInt(48));

    return [
        psyFelt(value),
        psyFelt(readBigIntU56FromBytesLE(publicKeyHash, 0)),
        psyFelt(readBigIntU56FromBytesLE(publicKeyHash, 7)),
        psyFelt(last48BitsWithFlag),
    ];
}

function getWithdrawalHashFromAddress(value: bigint, address: string) {
    const dAddress = getDecodedAddress(address);
    return getWithdrawalHashFromPublicKeyHash(value, dAddress.publicKeyHash, dAddress.scriptTypeFlag);
}

function getClaimDepositSigAction(request: IPsyClaimDepositRequest): IPsySigAction {
    return {
        network_magic: request.network_magic + "",
        user: request.user + "",
        sig_action: SIG_ACTION_CLAIM_DEPOSIT_MAGIC,
        nonce: "0",
        action_arguments: [
            ...hash256ToHashOut224(request.transaction_id).map((x) => x.toString()),
            request.amount + "",
            request.deposit_fee + "",
        ],
    };
}

function getTransferSigAction(request: IPsyTransferRequest): IPsySigAction {
    return {
        network_magic: request.network_magic + "",
        user: request.user + "",
        sig_action: SIG_ACTION_TRANSFER_MAGIC,
        nonce: request.nonce + "",
        action_arguments: [request.recipient + "", request.amount + ""],
    };
}

function getWithdrawalSigAction(request: IPsyWithdrawalRequest): IPsySigAction {
    const withdrawalHash = getWithdrawalHashFromAddress(psyFelt(BigInt(request.amount)), request.address);

    return {
        network_magic: request.network_magic + "",
        user: request.user + "",
        sig_action: SIG_ACTION_WITHDRAW_MAGIC,
        nonce: request.nonce + "",
        action_arguments: [
            withdrawalHash[0] + "",
            withdrawalHash[1] + "",
            withdrawalHash[2] + "",
            withdrawalHash[3] + "",
            request.withdrawal_fee + "",
        ],
    };
}

function computeSigActionHash(sigAction: IPsySigAction): IHashOut {
    const actionArgumentsHash = hashNoPad(sigAction.action_arguments.map((x) => psyFelt(x)));

    return hashNoPad([
        psyFelt(sigAction.network_magic),
        psyFelt(sigAction.user),
        psyFelt(sigAction.sig_action),
        psyFelt(sigAction.nonce),
        actionArgumentsHash[0],
        actionArgumentsHash[1],
        actionArgumentsHash[2],
        actionArgumentsHash[3],
    ]);
}
export {
    getWithdrawalHashFromPublicKeyHash,
    getWithdrawalHashFromAddress,
    getClaimDepositSigAction,
    getTransferSigAction,
    getWithdrawalSigAction,
    computeSigActionHash,
};
