import { hexToU8Array } from "doge-sdk";
import { hashNoPad } from "poseidon-goldilocks-lite";
import { bytes33ToPublicKeyFelts, hash256ToHashOut224, hashOutHex } from "./felt";
import { ICityL1Deposit } from "../rpc/baseTypes";

function getDepositHashHex(deposit: ICityL1Deposit): string {
    const txid224 = hash256ToHashOut224(deposit.txid);
    const value = BigInt(deposit.value);
    const publicKey = bytes33ToPublicKeyFelts(hexToU8Array(deposit.public_key));
    return hashOutHex(hashNoPad(txid224.concat([value]).concat(publicKey)));
}

export { getDepositHashHex };
