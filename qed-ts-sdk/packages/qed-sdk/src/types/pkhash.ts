import { twoToOneHex } from "poseidon-goldilocks-lite";
import { ZKPublicKeyInfo } from "./ZKPublicKeyInfo";



export function calculatePkHash(zkPublicKeyInfo: ZKPublicKeyInfo): string {
    return twoToOneHex(zkPublicKeyInfo.fingerprint, zkPublicKeyInfo.public_key_param);
}