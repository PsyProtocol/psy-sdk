import { decodeAddress, isP2PKHAddress } from "doge-sdk";

const WITHDRAWAL_TYPE_P2PKH = BigInt("0");
const WITHDRAWAL_TYPE_P2SH = BigInt("281474976710656");

function getDecodedAddress(address: string): { publicKeyHash: Uint8Array; scriptTypeFlag: number } {
    const { hash } = decodeAddress(address);
    if (isP2PKHAddress(address)) {
        return { publicKeyHash: hash, scriptTypeFlag: 0 };
    } else {
        return { publicKeyHash: hash, scriptTypeFlag: 1 };
    }
}

export { getDecodedAddress, WITHDRAWAL_TYPE_P2PKH, WITHDRAWAL_TYPE_P2SH };
