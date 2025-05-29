import { IHashOut, hashNoPad } from "poseidon-goldilocks-lite";
import { SIG_ACTION_CLAIM_DEPOSIT_MAGIC, SIG_ACTION_TRANSFER_MAGIC, SIG_ACTION_WITHDRAW_MAGIC } from "./constants";
import { ICityClaimDepositRequest, ICitySigAction, ICityTransferRequest, ICityWithdrawalRequest } from "./types";
import { SCNumberLike } from "../rpc/baseTypes";
import { getDecodedAddress } from "../utils/address";
import { readBigIntU48FromBytesLE, readBigIntU56FromBytesLE } from "../utils/data";
import { cityFelt, hash256ToHashOut224 } from "../utils/felt";

function getWithdrawalHashFromPublicKeyHash(
    value: bigint,
    publicKeyHash: Uint8Array,
    scriptTypeFlag: SCNumberLike
): IHashOut {
    const last48BitsWithFlag = readBigIntU48FromBytesLE(publicKeyHash, 14) | (BigInt(scriptTypeFlag) << BigInt(48));

    return [
        cityFelt(value),
        cityFelt(readBigIntU56FromBytesLE(publicKeyHash, 0)),
        cityFelt(readBigIntU56FromBytesLE(publicKeyHash, 7)),
        cityFelt(last48BitsWithFlag),
    ];
}

function getWithdrawalHashFromAddress(value: bigint, address: string) {
    const dAddress = getDecodedAddress(address);
    return getWithdrawalHashFromPublicKeyHash(value, dAddress.publicKeyHash, dAddress.scriptTypeFlag);
}

function getClaimDepositSigAction(request: ICityClaimDepositRequest): ICitySigAction {
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

function getTransferSigAction(request: ICityTransferRequest): ICitySigAction {
    return {
        network_magic: request.network_magic + "",
        user: request.user + "",
        sig_action: SIG_ACTION_TRANSFER_MAGIC,
        nonce: request.nonce + "",
        action_arguments: [request.recipient + "", request.amount + ""],
    };
}

function getWithdrawalSigAction(request: ICityWithdrawalRequest): ICitySigAction {
    const withdrawalHash = getWithdrawalHashFromAddress(cityFelt(BigInt(request.amount)), request.l1_address);

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

function computeSigActionHash(sigAction: ICitySigAction): IHashOut {
    const actionArgumentsHash = hashNoPad(sigAction.action_arguments.map((x) => cityFelt(x)));

    return hashNoPad([
        cityFelt(sigAction.network_magic),
        cityFelt(sigAction.user),
        cityFelt(sigAction.sig_action),
        cityFelt(sigAction.nonce),
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
