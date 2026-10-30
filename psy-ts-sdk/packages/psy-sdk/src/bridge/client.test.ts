import { describe, expect, it } from "@jest/globals";

import { PoseidonBridgeClient, WITHDRAWAL_LEAF_HASH_VECTOR, withdrawalLeafHashBytes32, withdrawalLeafNullifier } from "./client";
import type {
    IHTTPClient,
    ISimpleHTTPRequest,
    ISimpleHTTPResponse,
} from "../http/types";
import type {
    BridgeWithdrawalBatchGroth16Proof,
    BridgeWithdrawalBatchWitnessInput,
    DepositClaimProofResult,
    WithdrawalClaimProofQuery,
    WithdrawalClaimProofResult,
} from "./types";

/**
 * Minimal recording HTTP client: captures every request verbatim and returns
 * a fixed response. Lets us assert the exact wire shape the bridge client
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
function servicesOk<T>(data: T): ISimpleHTTPResponse {
    return { statusCode: 200, body: { success: true, data } };
}

describe("withdrawal leaf hash bytes", () => {
    it("renders low u32 then high u32, each big-endian", () => {
        expect(withdrawalLeafHashBytes32([
            0x1122334455667788n,
            0xaabbccddeeff0011n,
            0x0000000100000002n,
            0xffffffff00000000n,
        ])).toBe(WITHDRAWAL_LEAF_HASH_VECTOR);
    });
});

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

describe("PoseidonBridgeClient.getWithdrawalClaimProof", () => {
    const query: WithdrawalClaimProofQuery = {
        recipient: "0x1111111111111111111111111111111111111111",
        tokenAddress: "0x2222222222222222222222222222222222222222",
        amount: "777",
        nonce: "0x" + "ab".repeat(32),
        destinationChainIndex: 3,
        senderUserId: 42,
        tokenContractId: 7,
    };

    it.each([0, 42, 4294967295, "0", "42", "4294967295", "00042"])(
        "sends destination and the withdrawal leaf hash for sender %p",
        async (senderUserId) => {
            const { client, requests } = recordingHttpClient(servicesOk({ found: false, withdrawal: { recipient: query.recipient, token_address: query.tokenAddress, amount: query.amount, nonce: query.nonce, destination_chain_index: 3, sender_user_id: null, token_contract_id: null, nullifier: withdrawalLeafNullifier({ ...query, senderUserId, amount: query.amount, nonce: query.nonce }) } }));
            const bridgeClient = new PoseidonBridgeClient(client);
            const current = { ...query, senderUserId };
            await bridgeClient.getWithdrawalClaimProof("https://services.test/", current);
            const url = new URL(requests[0]!.url);
            expect(Array.from(url.searchParams.entries())).toEqual([
                ["destination_chain_index", "3"],
                ["nullifier", withdrawalLeafNullifier(current)],
            ]);
        },
    );

    it.each([
        undefined, null, -1, 1.5, 4294967296, NaN, Infinity,
        "", " ", "undefined", "null", "abc", "-1", "1.5", "4294967296",
        "1e2", "0x2a", "+42", " 42", "42 ", true, {}, 42n,
    ])("rejects invalid sender %p before making a request", async (senderUserId) => {
        const { client, requests } = recordingHttpClient(servicesOk({ found: false }));
        const bridgeClient = new PoseidonBridgeClient(client);
        // Exercise untyped JavaScript callers as well as valid TypeScript input.
        const invalidQuery = { ...query, senderUserId } as unknown as WithdrawalClaimProofQuery;

        await expect(bridgeClient.getWithdrawalClaimProof("https://services.test/", invalidQuery))
            .rejects.toThrow("withdrawal-claim-proof: senderUserId must be a uint32 number or unsigned decimal string");
        expect(requests).toHaveLength(0);
    });

    it("rejects an omitted sender before making a request", async () => {
        const { client, requests } = recordingHttpClient(servicesOk({ found: false }));
        const bridgeClient = new PoseidonBridgeClient(client);
        const { senderUserId: _senderUserId, ...missingSender } = query;

        await expect(bridgeClient.getWithdrawalClaimProof(
            "https://services.test/", missingSender as WithdrawalClaimProofQuery,
        )).rejects.toThrow("senderUserId must be a uint32 number or unsigned decimal string");
        expect(requests).toHaveLength(0);
    });

    it("rejects an echoed nullifier that is not the withdrawal leaf hash", async () => {
        const data: WithdrawalClaimProofResult = {
            found: true,
            withdrawal: {
                recipient: query.recipient,
                token_address: query.tokenAddress,
                amount: query.amount,
                nonce: query.nonce,
                destination_chain_index: 3,
                sender_user_id: 42,
                token_contract_id: 7,
                nullifier: "0x" + "cd".repeat(32),
            },
        };
        const { client } = recordingHttpClient(servicesOk(data));
        await expect(new PoseidonBridgeClient(client).getWithdrawalClaimProof("https://services.test/", query))
            .rejects.toThrow("echoed nullifier does not match the withdrawal leaf hash");
    });

    it("accepts a no-match whose nullifier echoes the queried leaf hash", async () => {
        const nullifier = withdrawalLeafNullifier(query);
        const data: WithdrawalClaimProofResult = {
            found: false,
            withdrawal: {
                recipient: null,
                token_address: null,
                amount: null,
                nonce: null,
                destination_chain_index: null,
                sender_user_id: null,
                token_contract_id: null,
                nullifier,
            },
        };
        const { client } = recordingHttpClient(servicesOk(data));
        await expect(new PoseidonBridgeClient(client).getWithdrawalClaimProof("https://services.test/", query)).resolves.toEqual(data);
    });
});

describe("PoseidonBridgeClient.proveWithdrawalClaim", () => {
    it("emits the batch JSON-RPC method and a single-element batch witness params array", async () => {
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
                    token_contract_id: 7,
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