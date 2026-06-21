import { hexToU8Array, u8ArrayToHex, hexToU8ArrayReversed } from '../node_modules/.pnpm/doge-sdk@0.4.5/node_modules/doge-sdk/dist/esm/utils/data.mjs';
import '../node_modules/.pnpm/doge-sdk@0.4.5/node_modules/doge-sdk/dist/esm/utils/random.mjs';
import '../node_modules/.pnpm/doge-sdk@0.4.5/node_modules/doge-sdk/dist/esm/hash/ripemd160.mjs';
import '../node_modules/.pnpm/doge-sdk@0.4.5/node_modules/doge-sdk/dist/esm/hash/sha256.mjs';
import '../node_modules/.pnpm/doge-sdk@0.4.5/node_modules/doge-sdk/dist/esm/address/network.mjs';
import '../node_modules/.pnpm/doge-sdk@0.4.5/node_modules/doge-sdk/dist/esm/transaction/transaction.mjs';
import '../node_modules/.pnpm/doge-sdk@0.4.5/node_modules/doge-sdk/dist/esm/script/opcodes.mjs';
import { hashOutToHex } from '../node_modules/.pnpm/poseidon-goldilocks-lite@0.2.3/node_modules/poseidon-goldilocks-lite/dist/poseidon-goldilocks-lite.esm.mjs';
import { cryptoRandomBytes } from './random.mjs';

const GOLDILOCKS_FP = BigInt("18446744069414584321");
function psyFelt(x) {
    return BigInt(x) % GOLDILOCKS_FP;
}
function cryptoRandomHashOut() {
    const data = new BigUint64Array(cryptoRandomBytes(32).buffer);
    return [data[0] % GOLDILOCKS_FP, data[1] % GOLDILOCKS_FP, data[2] % GOLDILOCKS_FP, data[3] % GOLDILOCKS_FP];
}
function cryptoRandomHashOutHex() {
    return hashOutHex(cryptoRandomHashOut());
}
function hashOutHex(hashOut) {
    return reverseHexBytes(hashOutToHex(hashOut).substring(2));
}
function hash256ToHashOut224(hashHex) {
    const base = new BigUint64Array(hexToU8Array(hashHex).buffer);
    return [
        base[0] & BigInt("0x00FFFFFFFFFFFFFF"),
        base[1] & BigInt("0x00FFFFFFFFFFFFFF"),
        base[2] & BigInt("0x00FFFFFFFFFFFFFF"),
        base[3] & BigInt("0x00FFFFFFFFFFFFFF"),
    ];
}
function publicKeyFeltsToBytes33(publicKey) {
    const result = new Uint8Array(33);
    result[0] = Number(publicKey[0]);
    const dv = new DataView(result.buffer);
    for (let i = 1; i < 9; i++) {
        dv.setUint32(i * 4 + 1, Number(publicKey[i]), true);
    }
    return result;
}
function bytes33ToPublicKeyFelts(bytes) {
    const result = [BigInt(bytes[0])];
    const dv = new DataView(bytes.buffer);
    for (let i = 1; i < 9; i++) {
        result[i] = BigInt(dv.getUint32(i * 4 + 1, true));
    }
    return result;
}
function reverseHexBytes(hex) {
    return u8ArrayToHex(hexToU8ArrayReversed(hex));
}
function trimTrailingZeroes(hex) {
    let i = hex.length - 1;
    while (i >= 0 && hex[i] === "0") {
        i--;
    }
    return hex.substring(0, i + 1);
}
function psyFeltSatsToDoge(x) {
    const decimalPart = BigInt(x) % BigInt(100000000);
    const integerPart = BigInt(x) / BigInt(100000000);
    if (decimalPart === BigInt(0)) {
        return integerPart.toString();
    }
    else {
        return integerPart + "." + trimTrailingZeroes(decimalPart.toString().padStart(8, "0"));
    }
}

export { bytes33ToPublicKeyFelts, cryptoRandomHashOut, cryptoRandomHashOutHex, hash256ToHashOut224, hashOutHex, psyFelt, psyFeltSatsToDoge, publicKeyFeltsToBytes33, reverseHexBytes };
