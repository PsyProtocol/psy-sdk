import { FetchHTTPClient } from "../http/fetchClient";
import type { IHTTPClient } from "../http/types";
import { hashNoPad } from "poseidon-goldilocks-lite";
import type {
    BridgeDepositBatchGroth16Proof,
    BridgeDepositBatchWitnessInput,
    BridgeWithdrawalBatchGroth16Proof,
    BridgeWithdrawalBatchWitnessInput,
    DepositClaimProofQuery,
    DepositClaimProofResult,
    JsonRpcResponse,
    ServicesApiResponse,
    U32x8,
    WithdrawalClaimProofQuery,
    WithdrawalClaimProofResult,
} from "./types";

function assertOkResponse<T>(
    body: ServicesApiResponse<T> | JsonRpcResponse<T> | null | undefined,
    context: string
): T {
    if (!body || typeof body !== "object") {
        throw new Error(`${context}: empty response`);
    }
    if ("success" in body) {
        if (!body.success) {
            throw new Error(`${context}: ${body.error || "unknown error"}`);
        }
        if (body.data === undefined) {
            throw new Error(`${context}: response missing data`);
        }
        return body.data;
    }
    if ("error" in body && body.error) {
        throw new Error(`${context}: ${body.error.message}`);
    }
    if (!("result" in body) || body.result === undefined) {
        throw new Error(`${context}: response missing result`);
    }
    return body.result;
}

function encodeQuery(params: Record<string, string>): string {
    return new URLSearchParams(params).toString();
}

export function u32x8ToHex(words: readonly number[]): string {
    if (words.length !== 8) {
        throw new Error(`expected 8 u32 words, got ${words.length}`);
    }
    let hex = "0x";
    for (let i = 0; i < 8; i++) {
        hex += (words[i] >>> 0).toString(16).padStart(8, "0");
    }
    return hex;
}

export function hexToU32x8(hex: string): U32x8 {
    const normalized = hex.startsWith("0x") ? hex.slice(2) : hex;
    if (normalized.length !== 64) {
        throw new Error(`expected 32-byte hex, got ${normalized.length / 2} bytes`);
    }
    const words = [] as number[];
    for (let i = 0; i < 8; i++) {
        words.push(Number.parseInt(normalized.slice(i * 8, (i + 1) * 8), 16) >>> 0);
    }
    return words as U32x8;
}
export const WITHDRAWAL_LEAF_HASH_VECTOR =
    "0x5566778811223344eeff0011aabbccdd000000020000000100000000ffffffff";

/** Render four Poseidon limbs as the indexed leaf-hash bytes32.
 * Limb 0x1122334455667788 begins 55667788, not 11223344. */
export function withdrawalLeafHashBytes32(limbs: readonly bigint[]): string {
    if (limbs.length !== 4) throw new Error("withdrawal leaf hash must be four limbs");
    let hex = "0x";
    for (const limb of limbs) {
        if (limb < 0n || limb > 0xffffffffffffffffn) throw new Error("withdrawal leaf hash limb is wider than a u64");
        const lo = limb & 0xffffffffn;
        const hi = limb >> 32n;
        hex += lo.toString(16).padStart(8, "0") + hi.toString(16).padStart(8, "0");
    }
    return hex;
}

function uint32Felt(value: number | string, name: string): bigint {
    const parsed = typeof value === "number"
        ? value
        : typeof value === "string" && /^[0-9]+$/.test(value)
            ? Number(value)
            : NaN;
    if (!Number.isInteger(parsed) || parsed < 0 || parsed > 0xffffffff) {
        throw new Error(`${name} must be a uint32 number or unsigned decimal string`);
    }
    return BigInt(parsed);
}

function bytes32FeltWords(value: bigint | string): bigint[] {
    const parsed = typeof value === "bigint" ? value : BigInt(value);
    if (parsed < 0n || parsed >= 1n << 256n) throw new Error("withdrawal value is not a uint256");
    const words: bigint[] = [];
    for (let shift = 224; shift >= 0; shift -= 32) words.push((parsed >> BigInt(shift)) & 0xffffffffn);
    return words;
}

function addressFeltWords(value: string): bigint[] {
    const raw = value.trim().replace(/^0x/i, "");
    if (!/^[0-9a-fA-F]{40}$/.test(raw) && !(raw.length === 64 && /^0{24}/i.test(raw))) {
        throw new Error("withdrawal address is not 20 bytes");
    }
    return bytes32FeltWords(BigInt(`0x${raw.length === 40 ? raw : raw.slice(24)}`));
}

export function withdrawalLeafNullifier(identity: {
    senderUserId: number | string;
    recipient: string;
    tokenAddress: string;
    amount: bigint | string;
    nonce: bigint | string;
    destinationChainIndex: number | string;
    tokenContractId: number | string;
}): string {
    const felts = [
        uint32Felt(identity.senderUserId, "senderUserId"),
        ...addressFeltWords(identity.recipient),
        ...addressFeltWords(identity.tokenAddress),
        ...bytes32FeltWords(identity.amount),
        ...bytes32FeltWords(identity.nonce),
        uint32Felt(identity.destinationChainIndex, "destinationChainIndex"),
        uint32Felt(identity.tokenContractId, "tokenContractId"),
    ];
    return withdrawalLeafHashBytes32(hashNoPad(felts) as readonly bigint[]);
}


export class PoseidonBridgeClient {
    private readonly httpClient: IHTTPClient;

    constructor(httpClient?: IHTTPClient) {
        this.httpClient = httpClient ?? new FetchHTTPClient();
    }

    async getDepositClaimProof(servicesUrl: string, query: DepositClaimProofQuery): Promise<DepositClaimProofResult> {
        const qs = encodeQuery({
            deposit_index: query.depositIndex.toString(),
            source_chain_index: query.sourceChainIndex.toString(),
            snapshot_deposit_count: query.snapshotDepositCount.toString(),
        });
        const resp = await this.httpClient.sendRequest({
            url: `${servicesUrl.replace(/\/$/, "")}/api/v1/bridge/deposit-claim-proof?${qs}`,
            method: "GET",
            responseType: "json",
        });
        return assertOkResponse(resp.body, "deposit-claim-proof");
    }

    async getWithdrawalClaimProof(
        servicesUrl: string,
        query: WithdrawalClaimProofQuery
    ): Promise<WithdrawalClaimProofResult> {
        let nullifier: string;
        try {
            nullifier = withdrawalLeafNullifier({
                senderUserId: query.senderUserId,
                recipient: query.recipient,
                tokenAddress: query.tokenAddress,
                amount: query.amount,
                nonce: query.nonce,
                destinationChainIndex: query.destinationChainIndex,
                tokenContractId: query.tokenContractId,
            });
        } catch (error) {
            throw new Error(`withdrawal-claim-proof: ${error instanceof Error ? error.message : String(error)}`);
        }
        const qs = encodeQuery({
            destination_chain_index: query.destinationChainIndex.toString(),
            nullifier,
        });
        const resp = await this.httpClient.sendRequest({
            url: `${servicesUrl.replace(/\/$/, "")}/api/v1/bridge/withdrawal-claim-proof?${qs}`,
            method: "GET",
            responseType: "json",
        });
        const result = assertOkResponse<WithdrawalClaimProofResult>(resp.body, "withdrawal-claim-proof");
        if (result.withdrawal?.nullifier?.toLowerCase() !== nullifier.toLowerCase()) {
            throw new Error("withdrawal-claim-proof: echoed nullifier does not match the withdrawal leaf hash");
        }
        if (result.leaf_hash != null && result.leaf_hash.toLowerCase() !== nullifier.toLowerCase()) {
            throw new Error("withdrawal-claim-proof: leaf_hash does not match the withdrawal leaf hash");
        }
        return result;
    }

    async proveWithdrawalClaim(
        proveProxyUrl: string,
        witnessInput: BridgeWithdrawalBatchWitnessInput
    ): Promise<BridgeWithdrawalBatchGroth16Proof> {
        const resp = await this.httpClient.sendRequest({
            url: proveProxyUrl,
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({
                jsonrpc: "2.0",
                id: 1,
                method: "psy_prove_withdrawal_batch_claim_groth16",
                params: [witnessInput],
            }),
            responseType: "json",
        });
        return assertOkResponse(resp.body, "psy_prove_withdrawal_batch_claim_groth16");
    }

    async proveDepositBatchAppend(
        proveProxyUrl: string,
        witnessInput: BridgeDepositBatchWitnessInput
    ): Promise<BridgeDepositBatchGroth16Proof> {
        const resp = await this.httpClient.sendRequest({
            url: proveProxyUrl,
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({
                jsonrpc: "2.0",
                id: 1,
                method: "psy_prove_deposit_batch_append_groth16",
                params: [witnessInput],
            }),
            responseType: "json",
        });
        return assertOkResponse(resp.body, "psy_prove_deposit_batch_append_groth16");
    }
}
