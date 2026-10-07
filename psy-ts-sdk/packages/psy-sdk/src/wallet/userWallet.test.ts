import { describe, expect, it } from "@jest/globals";
import type { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { PsyUserWallet } from "./userWallet";

const fixedLeaf = {
    public_key: "pk-cross",
    user_state_tree_root: "root-9",
    balance: 17n,
    nonce: 4n,
    last_checkpoint_id: 3n,
    event_index: 1n,
    user_id: 9n,
};

function realm(servesUser: boolean): IRealmEdgeRpcProvider {
    return {
        getLatestBlockState: async () => ({ checkpoint_id: 8 }),
        getUserLeafData: async () => {
            if (!servesUser) throw new Error("user is not in this realm");
            return fixedLeaf;
        },
        getRpcProviderByUserId() {
            return this;
        },
    } as unknown as IRealmEdgeRpcProvider;
}

describe("PsyUserWallet cross-realm identity", () => {
    it("rejects the unresolved realm and reads the fixed leaf from the resolved realm", async () => {
        const unresolved = realm(false);
        const resolved = realm(true);
        const selected = new PsyUserWallet(
            "regtest",
            { getPublicKeyHex: async () => "pk-cross" } as never,
            { getUserId: async () => 9 } as never,
            unresolved,
            unresolved,
            0,
            "pk-cross",
            false,
        );
        await expect(selected.refresh()).resolves.toMatchObject({ user_id: 0n, balance: 0n });
        expect(selected.status).toBe(false);

        const routed = new PsyUserWallet(
            "regtest",
            { getPublicKeyHex: async () => "pk-cross", deployContract: async () => "submitted" } as never,
            { getUserId: async () => 9 } as never,
            { getRpcProviderByUserId: (userId: number) => (userId === 9 ? resolved : unresolved) } as never,
            unresolved,
            0,
            "pk-cross",
            false,
        );
        await expect(routed.refresh()).resolves.toMatchObject({
            user_id: fixedLeaf.user_id,
            balance: fixedLeaf.balance,
            nonce: fixedLeaf.nonce,
        });
        expect(routed.status).toBe(true);
        expect(routed.userId).toBe(9);
        await expect(routed.deployContract([])).resolves.toBe("submitted");
    });

    it("does not deploy when refresh cannot resolve the user", async () => {
        const deployed: string[] = [];
        const unresolved = new PsyUserWallet(
            "regtest",
            {
                getPublicKeyHex: async () => "pk-cross",
                deployContract: async (deployerUserId: string) => {
                    deployed.push(deployerUserId);
                    return "submitted";
                },
            } as never,
            { getUserId: async () => { throw new Error("user is not indexed"); } } as never,
            realm(true),
            realm(false),
            0,
            "pk-cross",
            false,
        );

        await expect(unresolved.deployContract([])).rejects.toThrow("wallet user id is unresolved");
        expect(unresolved.status).toBe(false);
        expect(deployed).toEqual([]);
    });
});
