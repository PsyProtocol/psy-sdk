import { describe, expect, it, jest } from "@jest/globals";
import { Contract } from "../../../contract-sdk/src/runtime/contract";
import { Signer } from "../../../contract-sdk/src/runtime/types";
import type { AbiInput } from "../../../contract-sdk/src/types/abi-format";

const VIEW_ABI: AbiInput = {
    schema_version: "2.0.0",
    contract: {
        name: "ViewIdentity",
        state_tree_height: 1,
        state: [],
        methods: [{
            name: "get_value",
            method_id: 1,
            state_mutability: "view",
            inputs: [],
            outputs: [{
                name: "value",
                type: { kind: "primitive", name: "Felt" },
                felt_size: 1,
            }],
            input_felt_count: 0,
            output_felt_count: 1,
        }],
    },
    types: [],
};

describe("Contract view identity", () => {
    it("passes the attached signer public key to callViewFunction", async () => {
        const provider = {
            getContractState: jest.fn(async () => []),
            callViewFunction: jest.fn(async (_contractId: unknown, _method: string, _args: unknown[], _publicKey: string) => [7n]),
        };
        const contract = new Contract(
            4n,
            VIEW_ABI,
            new Signer("pk-b", provider as never),
            { checkpointId: 3n, userId: 2n },
        );

        await expect(contract.callMethod("get_value")).resolves.toBe(7n);
        expect(provider.callViewFunction).toHaveBeenCalledWith(4n, "get_value", [], "pk-b");
    });

    it("rejects a provider-only view because no user identity is available", async () => {
        const provider = {
            getContractState: jest.fn(async () => []),
            callViewFunction: jest.fn(async (_contractId: unknown, _method: string, _args: unknown[], _publicKey: string) => [7n]),
        };
        const contract = new Contract(4n, VIEW_ABI, provider, { checkpointId: 3n, userId: 2n });

        await expect(contract.callMethod("get_value")).rejects.toThrow(
            "Signer required for view functions. Use contract.attach(signer)",
        );
        expect(provider.callViewFunction).not.toHaveBeenCalled();
    });

    it("never falls back to a transaction for a view", async () => {
        const provider = {
            getContractState: jest.fn(async () => []),
            sendTransaction: jest.fn(async () => []),
        };
        const contract = new Contract(
            4n,
            VIEW_ABI,
            new Signer("pk-b", provider as never),
            { checkpointId: 3n, userId: 2n },
        );

        await expect(contract.callMethod("get_value")).rejects.toThrow();
        expect(provider.sendTransaction).not.toHaveBeenCalled();
    });
});
