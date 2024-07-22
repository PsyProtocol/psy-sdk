import { IHashOut, hashOutToHex, hexToHashOut } from "poseidon-goldilocks-lite";
import { SCNumberLike } from "../rpc/baseTypes";
import { cryptoRandomBytes } from "./random";
import { hexToU8Array, hexToU8ArrayReversed, u8ArrayToHex } from "doge-sdk";

const GOLDILOCKS_FP = BigInt("18446744069414584321");
function cityFelt(x: SCNumberLike): bigint {
  return BigInt(x) % GOLDILOCKS_FP;
}
function cryptoRandomHashOut(): IHashOut {
  const data = new BigUint64Array(cryptoRandomBytes(32).buffer)
  return [
    data[0]%GOLDILOCKS_FP,
    data[1]%GOLDILOCKS_FP,
    data[2]%GOLDILOCKS_FP,
    data[3]%GOLDILOCKS_FP,
  ];
}

function cryptoRandomHashOutHex(): string {
  return hashOutHex(cryptoRandomHashOut());
}
function hashOutHex(hashOut: IHashOut): string {
  return reverseHexBytes(hashOutToHex(hashOut).substring(2));
}
function hash256ToHashOut224(hashHex: string): IHashOut {
  const base= new BigUint64Array(hexToU8Array(hashHex).buffer);
  return [
    base[0]&BigInt("0x00FFFFFFFFFFFFFF"),
    base[1]&BigInt("0x00FFFFFFFFFFFFFF"),
    base[2]&BigInt("0x00FFFFFFFFFFFFFF"),
    base[3]&BigInt("0x00FFFFFFFFFFFFFF"),
  ];
}


function reverseHexBytes(hex: string): string {
  return u8ArrayToHex(hexToU8ArrayReversed(hex));

}

export {
  cityFelt,
  cryptoRandomHashOut,
  hashOutHex,
  cryptoRandomHashOutHex,
  hash256ToHashOut224,
  reverseHexBytes,
}