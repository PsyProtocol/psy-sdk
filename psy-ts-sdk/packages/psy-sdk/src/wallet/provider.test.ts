import { describe, expect, it, jest } from "@jest/globals";
import type { ViewCallData, ViewCallResult } from "../local-prover-rpc";

jest.mock("../local-web-prover", () => ({
    PsyWasmWebProverProvider: class {},
}));

import { PsyUserWalletProvider } from "./provider";

describe("PsyUserWalletProvider.callViewFunction", () => {
    it("executes a view through the prover for the requested signer", async () => {
        const callView = jest.fn(async (
            _pkHash: string,
            _callData: ViewCallData,
        ): Promise<ViewCallResult> => ({
            checkpoint_id: 9,
            contract_calls: [{
                contract_id: 7,
                method_name: "get_counter",
                inputs: [3],
                outputs: [42],
            }],
            storage_reads: [],
        }));
        const signerA = { getPublicKeyHex: jest.fn(async () => "pk-a") };
        const signerB = { getPublicKeyHex: jest.fn(async () => "pk-b") };
        const getSignerByPublicKeyHex = jest.fn(async (publicKey: string) => {
            if (publicKey === "pk-b") return signerB;
            throw new Error(`Unknown signer ${publicKey}`);
        });
        const provider = new PsyUserWalletProvider(
            "regtest",
            {} as never,
            {} as never,
            {
                getSigners: jest.fn(async () => [signerA, signerB]),
                getSignerByPublicKeyHex,
            } as never,
            { callView } as never,
        );

        await expect(provider.callViewFunction(7, "get_counter", [3], "pk-b")).resolves.toEqual([42]);
        expect(getSignerByPublicKeyHex).toHaveBeenCalledWith("pk-b");
        expect(callView).toHaveBeenCalledWith("pk-b", {
            contract_calls: [{
                contract_id: 7n,
                method_name: "get_counter",
                inputs: [3n],
            }],
        });
    });

    it("fails clearly when no wallet identity is supplied", async () => {
        const provider = new PsyUserWalletProvider(
            "regtest",
            {} as never,
            {} as never,
            {} as never,
            {} as never,
        );

        await expect(provider.callViewFunction(7, "get_counter", [], "")).rejects.toThrow(
            'Cannot call view function "get_counter": a wallet public key is required',
        );
    });
});
