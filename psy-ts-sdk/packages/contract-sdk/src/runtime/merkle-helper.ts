import { keccak256, toBeHex, zeroPadValue } from "ethers";
import { Felt, IContractProvider } from "./types";
import { IMerkleProxyHelper, wrapMerkleProxyHelperBasicSimplifier } from "./proxy";

export function createMerkleHelper(
    provider: IContractProvider,
    checkpointId: Felt,
    contractId: Felt,
    userId: Felt
): IMerkleProxyHelper {
    const baseHelper: IMerkleProxyHelper = {
        add: (a: any, b: any) => BigInt(a) + BigInt(b),
        mul: (a: any, b: any) => BigInt(a) * BigInt(b),
        simplify: (x: any) => {
            if (typeof x === "bigint") return x;
            if (typeof x === "number") return BigInt(x);
            if (typeof x === "string" && /^\d+$/.test(x)) return BigInt(x);
            return x;
        },
        getHashFelt: async (index: any) => {
            const offset = calculateOffset(index);
            const data = await provider.getContractState(checkpointId, contractId, userId, [offset]);
            return data[0] || BigInt(0);
        },
        setHashFelt: async (_index: any, _value: any) => {
            throw new Error("Direct state writes not supported");
        },
        resolveFelt: (value: any) => BigInt(value),
    };

    return wrapMerkleProxyHelperBasicSimplifier(baseHelper);
}

export function calculateOffset(index: any): Felt {
    if (typeof index === "bigint") return index;
    if (typeof index === "number") return BigInt(index);
    if (typeof index === "object" && index.base !== undefined && index.key !== undefined) {
        return keccak256Felt(index.key, index.base);
    }
    return BigInt(index);
}

export function keccak256Felt(key: Felt, base: Felt): Felt {
    const keyBytes = zeroPadValue(toBeHex(key), 32);
    const baseBytes = zeroPadValue(toBeHex(base), 32);
    const encoded = keyBytes + baseBytes.slice(2);
    return BigInt(keccak256(encoded));
}
