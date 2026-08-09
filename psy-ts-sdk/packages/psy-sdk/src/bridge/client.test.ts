import { describe, expect, it } from "@jest/globals";

import { PoseidonBridgeClient } from "./client";
import type {
    IHTTPClient,
    ISimpleHTTPRequest,
    ISimpleHTTPResponse,
} from "../http/types";
import type {
    BridgeWithdrawalBatchGroth16Proof,
    BridgeWithdrawalBatchWitnessInput,
    DepositClaimProofResult,
} from "./types";

/**
 * Minimal recording HTTP client: captures every request verbatim and returns
 * a fixed response. Lets us assert the exact wire shape getDepositClaimProof
 * emits, without touching the network or the real fetch implementation.
 */
function recordingHttpClient(response: ISimpleHTTPResponse): {
    client: IHTTPClient;
    requests: ISimpleHTTPRequest[];
} {
    const requests: ISimpleHTTPRequest[] = [];
    const client: IHTTPClient = {
        async sendRequest(request: ISimpleHTTPRequest): Promise<ISimpleHTTPResponse> {
            requests.push(request);
            return response;
        },
    };
    return { client, requests };
}
/** JSON-RPC 2.0 success envelope shape consumed by assertOkResponse. */
function jsonRpcOk<T>(result: T): ISimpleHTTPResponse {
    return { statusCode: 200, body: { jsonrpc: "2.0", id: 1, result } };
}

/** Services success envelope shape consumed by assertOkResponse. */
function servicesOk(data: DepositClaimProofResult): ISimpleHTTPResponse {
    return { statusCode: 200, body: { success: true, data } };
}

describe("PoseidonBridgeClient.getDepositClaimProof", () => {
    it("serializes exactly deposit_index, source_chain_index and snapshot_deposit_count into the request URL", async () => {
        const { client, requests } = recordingHttpClient(
            servicesOk({ found: false }),
        );
        const bridgeClient = new PoseidonBridgeClient(client);

        await bridgeClient.getDepositClaimProof("https://services.test/", {
            depositIndex: 104,
            sourceChainIndex: 0,
            snapshotDepositCount: 5n,
        });

        expect(requests).toHaveLength(1);
        const req = requests[0]!;
        expect(req.method).toBe("GET");
        expect(req.responseType).toBe("json");

        // The full request URL must carry only the three current-contract
        // params — no depositor / nonce / source_chain_id anywhere.
        expect(req.url).toBe(
            "https://services.test/api/v1/bridge/deposit-claim-proof" +
                "?deposit_index=104&source_chain_index=0&snapshot_deposit_count=5",
        );
        // Negative guard: obsolete request keys must never appear.
        expect(req.url).not.toContain("depositor");
        expect(req.url).not.toContain("nonce");
        expect(req.url).not.toContain("source_chain_id");

        // Cross-check via parsed query params (order-independent).
        const parsed = new URL(req.url);
        expect(parsed.pathname).toBe("/api/v1/bridge/deposit-claim-proof");
        expect(parsed.searchParams.get("deposit_index")).toBe("104");
        expect(parsed.searchParams.get("source_chain_index")).toBe("0");
        expect(parsed.searchParams.get("snapshot_deposit_count")).toBe("5");
        expect(Array.from(parsed.searchParams.keys()).sort()).toEqual([
            "deposit_index",
            "snapshot_deposit_count",
            "source_chain_index",
        ]);
    });

    it("strips a trailing slash from the services URL and stringifies bigint params", async () => {
        const { client, requests } = recordingHttpClient(
            servicesOk({ found: false }),
        );
        const bridgeClient = new PoseidonBridgeClient(client);

        await bridgeClient.getDepositClaimProof("https://services.test", {
            depositIndex: 7n,
            sourceChainIndex: 31337n,
            snapshotDepositCount: "12",
        });

        expect(requests[0]!.url).toBe(
            "https://services.test/api/v1/bridge/deposit-claim-proof" +
                "?deposit_index=7&source_chain_index=31337&snapshot_deposit_count=12",
        );
    });

    it("preserves the dual-index response (global deposit_index vs chain_local_deposit_index)", async () => {
        // The services contract returns deposit_index as the Envio global
        // locator and chain_local_deposit_index as the per-source-chain tree
        // leaf index (= depositProofRaw.deposit_index used by the L2 claim).
        const data: DepositClaimProofResult = {
            found: true,
            deposit_index: 104,
            chain_local_deposit_index: 4,
            source_chain_index: 0,
            snapshot_deposit_count: 5,
            leaf_hash: "0xleaf",
            siblings: ["0xsib0", "0xsib1"],
            deposit_root: "0xroot",
            deposit: {
                shield_address: "0xshield",
                token_address: "0xtoken",
                l2_token_contract_id: "1",
                amount: "777",
                note_commitment: "0xnote",
                source_chain_id: 0,
            },
        };
        const { client, requests } = recordingHttpClient(servicesOk(data));
        const bridgeClient = new PoseidonBridgeClient(client);

        const result = await bridgeClient.getDepositClaimProof(
            "https://services.test/",
            {
                depositIndex: 104,
                sourceChainIndex: 0,
                snapshotDepositCount: 5,
            },
        );

        expect(result).toEqual(data);
        // Dual-index invariant: the global locator and the chain-local index
        // are distinct and both carried through unchanged.
        expect(result.deposit_index).toBe(104);
        expect(result.chain_local_deposit_index).toBe(4);
        expect(result.deposit_index).not.toBe(result.chain_local_deposit_index);

        // Request still only carries the three current-contract params.
        const url = requests[0]!.url;
        expect(url).toContain("deposit_index=104");
        expect(url).toContain("source_chain_index=0");
        expect(url).toContain("snapshot_deposit_count=5");
        expect(url).not.toContain("source_chain_id");
    });

    it("surfaces the services error envelope instead of a deposit proof", async () => {
        const { client } = recordingHttpClient({
            statusCode: 500,
            body: { success: false, error: "deposit not indexed yet" },
        });
        const bridgeClient = new PoseidonBridgeClient(client);

        await expect(
            bridgeClient.getDepositClaimProof("https://services.test/", {
                depositIndex: 104,
                sourceChainIndex: 0,
                snapshotDepositCount: 5,
            }),
        ).rejects.toThrow(/deposit not indexed yet/);
    });
});

describe("PoseidonBridgeClient.proveWithdrawalClaim", () => {
    it("emits the canonical batch JSON-RPC method and a single-element batch witness params array", async () => {
        const proof: BridgeWithdrawalBatchGroth16Proof = {
            solidity_proof: ["0xp0", "0xp1", "0xp2", "0xp3", "0xp4", "0xp5", "0xp6", "0xp7"],
            public_inputs: [1, 2, 3],
            slot_data: [9, 8, 7],
        };
        const { client, requests } = recordingHttpClient(jsonRpcOk(proof));
        const bridgeClient = new PoseidonBridgeClient(client);

        const witnessInput: BridgeWithdrawalBatchWitnessInput = {
            bridge_user_id: 42,
            withdrawals: [
                {
                    withdrawal_root: "0xroot",
                    sender_user_id: 7,
                    recipient: [10],
                    token: [11],
                    amount: [12],
                    nonce: [13],
                    destination_chain_index: 0,
                    leaf_index: 4,
                    bridge_user_id: 42,
                    siblings: ["0xsib0", "0xsib1"],
                },
            ],
        };

        const result = await bridgeClient.proveWithdrawalClaim(
            "https://prove-proxy.test/",
            witnessInput,
        );

        // Returned proof passes through unchanged from the JSON-RPC result.
        expect(result).toEqual(proof);

        expect(requests).toHaveLength(1);
        const req = requests[0]!;
        expect(req.method).toBe("POST");
        expect(req.responseType).toBe("json");
        expect(req.headers?.["content-type"]).toBe("application/json");

        // The body is a JSON-RPC 2.0 envelope; parse it to assert the exact
        // method name and params wire shape.
        const body = JSON.parse(req.body as string) as {
            jsonrpc: string;
            id: number;
            method: string;
            params: unknown[];
        };
        expect(body.jsonrpc).toBe("2.0");
        expect(body.id).toBe(1);
        expect(body.method).toBe("psy_prove_withdrawal_batch_claim_groth16");
        // Canonical batch wrapper: params is a single-element array wrapping
        // the batch witness input (bridge_user_id + withdrawals[]).
        expect(body.params).toEqual([witnessInput]);

        // Negative guard: the legacy non-batch method name must never regress.
        expect(body.method).not.toBe("psy_prove_batch_withdrawal_groth16");
        // The batch witness carries the withdrawals array, never a bare
        // single-withdrawal input at the params top level.
        expect(Array.isArray((body.params[0] as BridgeWithdrawalBatchWitnessInput).withdrawals)).toBe(true);
    });
});