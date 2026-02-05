import { hexToU8Array, hexToU8ArrayReversed, u8ArrayToHex } from "doge-sdk";
import { IHashOut, hashOutToHex } from "poseidon-goldilocks-lite";
import { cryptoRandomBytes } from "./random";
import { SCNumberLike } from "../core";

const GOLDILOCKS_FP = BigInt("18446744069414584321");
function psyFelt(x: SCNumberLike): bigint {
    return BigInt(x) % GOLDILOCKS_FP;
}
function cryptoRandomHashOut(): IHashOut {
    const data = new BigUint64Array(cryptoRandomBytes(32).buffer);
    return [data[0] % GOLDILOCKS_FP, data[1] % GOLDILOCKS_FP, data[2] % GOLDILOCKS_FP, data[3] % GOLDILOCKS_FP];
}

function cryptoRandomHashOutHex(): string {
    return hashOutHex(cryptoRandomHashOut());
}
function hashOutHex(hashOut: IHashOut): string {
    return reverseHexBytes(hashOutToHex(hashOut).substring(2));
}
function hash256ToHashOut224(hashHex: string): IHashOut {
    const base = new BigUint64Array(hexToU8Array(hashHex).buffer);
    return [
        base[0] & BigInt("0x00FFFFFFFFFFFFFF"),
        base[1] & BigInt("0x00FFFFFFFFFFFFFF"),
        base[2] & BigInt("0x00FFFFFFFFFFFFFF"),
        base[3] & BigInt("0x00FFFFFFFFFFFFFF"),
    ];
}

function publicKeyFeltsToBytes33(publicKey: SCNumberLike[]): Uint8Array {
    const result = new Uint8Array(33);
    result[0] = Number(publicKey[0]);
    const dv = new DataView(result.buffer);
    for (let i = 1; i < 9; i++) {
        dv.setUint32(i * 4 + 1, Number(publicKey[i]), true);
    }
    return result;
}
function bytes33ToPublicKeyFelts(bytes: Uint8Array): bigint[] {
    const result: bigint[] = [BigInt(bytes[0])];
    const dv = new DataView(bytes.buffer);

    for (let i = 1; i < 9; i++) {
        result[i] = BigInt(dv.getUint32(i * 4 + 1, true));
    }
    return result;
}

function reverseHexBytes(hex: string): string {
    return u8ArrayToHex(hexToU8ArrayReversed(hex));
}
function trimTrailingZeroes(hex: string): string {
    let i = hex.length - 1;
    while (i >= 0 && hex[i] === "0") {
        i--;
    }
    return hex.substring(0, i + 1);
}
function psyFeltSatsToDoge(x: SCNumberLike): string {
    const decimalPart = BigInt(x) % BigInt(100_000_000);
    const integerPart = BigInt(x) / BigInt(100_000_000);
    if (decimalPart === BigInt(0)) {
        return integerPart.toString();
    } else {
        return integerPart + "." + trimTrailingZeroes(decimalPart.toString().padStart(8, "0"));
    }
}

export {
    psyFelt,
    cryptoRandomHashOut,
    hashOutHex,
    cryptoRandomHashOutHex,
    hash256ToHashOut224,
    reverseHexBytes,
    psyFeltSatsToDoge,
    publicKeyFeltsToBytes33,
    bytes33ToPublicKeyFelts,
};
