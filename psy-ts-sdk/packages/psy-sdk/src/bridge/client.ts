import { FetchHTTPClient } from "../http/fetchClient";
import type { IHTTPClient } from "../http/types";
import type {
    BridgeDepositBatchGroth16Proof,
    BridgeDepositBatchWitnessInput,
    BridgeWithdrawalGroth16Proof,
    BridgeWithdrawalWitnessInput,
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
        const params: Record<string, string> = {
            recipient: query.recipient,
            token_address: query.tokenAddress,
            amount: query.amount,
            nonce: query.nonce,
            destination_chain_index: query.destinationChainIndex.toString(),
        };
        if (query.senderUserId !== undefined) {
            params.sender_user_id = query.senderUserId.toString();
        }
        const qs = encodeQuery(params);
        const resp = await this.httpClient.sendRequest({
            url: `${servicesUrl.replace(/\/$/, "")}/api/v1/bridge/withdrawal-claim-proof?${qs}`,
            method: "GET",
            responseType: "json",
        });
        return assertOkResponse(resp.body, "withdrawal-claim-proof");
    }

    async proveWithdrawalClaim(
        proveProxyUrl: string,
        witnessInput: BridgeWithdrawalWitnessInput
    ): Promise<BridgeWithdrawalGroth16Proof> {
        const resp = await this.httpClient.sendRequest({
            url: proveProxyUrl,
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({
                jsonrpc: "2.0",
                id: 1,
                method: "psy_prove_batch_withdrawal_groth16",
                params: [witnessInput],
            }),
            responseType: "json",
        });
        return assertOkResponse(resp.body, "psy_prove_batch_withdrawal_groth16");
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
