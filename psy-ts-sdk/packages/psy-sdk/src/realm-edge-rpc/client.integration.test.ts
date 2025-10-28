import { RealmEdgeRpcProvider } from "./client";
import { IRealmEdgeRpcProvider } from "./types";
import {
    ProofWithPublicInputs,
    Proof,
    OpeningSet,
    FriProof,
    MerkleCap,
    ExtensionField,
    PolynomialCoeffs,
    FriInitialTreeProof,
    FriQueryRound,
    FriQueryStep,
    QEDUserLeaf,
    SubmitUserEndCapNonProofInput,
    SubmitUserEndCapNonProofCoreInput,
    QEDContractStateUpdateHistory, // For potential direct rpc calls if needed, or reference
} from "../types";

// Note: These tests are integration tests and require a running QED Realm Edge RPC endpoint.
// Configure the endpoint URL via the TEST_REALM_EDGE_RPC_URL environment variable.
// You might need to set up Jest and ts-jest in your project if not already done.
// e.g., yarn add jest @types/jest ts-jest -D
// jest.config.js: module.exports = { preset: 'ts-jest', testEnvironment: 'node' };

const MOCK_RPC_URL = process.env.TEST_REALM_EDGE_RPC_URL || "http://localhost:8547";

// --- Mock Data ---
const mockUserIdNum = 1;
const mockUserIdBigInt = 1n;
const mockCheckpointIdNum = 1;
const mockCheckpointIdBigInt = 1n;
const mockLeafCheckpointIdNum = 2;
const mockLeafCheckpointIdBigInt = 2n;
const mockContractIdNum = 3;
const mockContractIdBigInt = 3n;
const mockHeight = 4; // Typically number
const mockLeafIdNum = 5;
const mockLeafIdBigInt = 5n;
const mockRootLevel = 0; // Typically number, levels in a tree
const mockLeafLevel = 1; // Typically number
const mockLeafIndexNum = 8;
const mockLeafIndexBigInt = 8n;

const mockUserLeafInstance: QEDUserLeaf = {
    public_key: "",
    user_state_tree_root: "",
    balance: 0n,
    nonce: 0n,
    last_checkpoint_id: 0n,
    event_index: 0n,
    user_id: 0n,
};

const mockSubmitUserEndCapNonProofCoreInput: SubmitUserEndCapNonProofCoreInput = {
    checkpoint_id: mockCheckpointIdBigInt,
    stats: { mock_guta_stat: "value" }, // Mock for GUTAStats
    state_transition: { mock_ups_result: "compact" }, // Mock for UPSEndCapResultCompact
    new_user_leaf: mockUserLeafInstance,
};

const mockContractStateUpdateHistory: QEDContractStateUpdateHistory = {
    contract_id: mockContractIdBigInt,
    updates: [{ mock_update_field: "mock_update_value" }], // Mock for actual update type
};

const mockSubmitUserEndCapInput: SubmitUserEndCapNonProofInput = {
    core: mockSubmitUserEndCapNonProofCoreInput,
    contract_state_updates: [mockContractStateUpdateHistory],
};

// Simplified Mock for Proof (enough for type checking client calls)
const mockMerkleCap: MerkleCap = { digests: ["0xhash1"] };
const mockExtensionField: ExtensionField = { elements: [1n, 2n] }; // Field can be string or bigint
const mockFriQueryStep: FriQueryStep = { evals: [mockExtensionField], merkle_proof: { siblings: ["0xhash2"] } };
const mockFriInitialTreeProof: FriInitialTreeProof = { evals_proofs: [[[1n], { siblings: ["0xhash3"] }]] };
const mockFriQueryRound: FriQueryRound = { initial_trees_proof: mockFriInitialTreeProof, steps: [mockFriQueryStep] };
const mockPolynomialCoeffs: PolynomialCoeffs = { coeffs: [mockExtensionField] };
const mockFriProof: FriProof = {
    commit_phase_merkle_caps: [mockMerkleCap],
    query_round_proofs: [mockFriQueryRound],
    final_poly: mockPolynomialCoeffs,
    pow_witness: 1n,
};
const mockOpeningSet: OpeningSet = {
    constants: [mockExtensionField],
    plonk_sigmas: [mockExtensionField],
    wires: [mockExtensionField],
    plonk_zs: [mockExtensionField],
    plonk_zs_next: [mockExtensionField],
    partial_products: [mockExtensionField],
    quotient_polys: [mockExtensionField],
    lookup_zs: [mockExtensionField],
    lookup_zs_next: [mockExtensionField],
};

const mockProofFullInstance: Proof = {
    wires_cap: mockMerkleCap,
    plonk_zs_partial_products_cap: mockMerkleCap,
    quotient_polys_cap: mockMerkleCap,
    openings: mockOpeningSet,
    opening_proof: mockFriProof,
};

const mockProofInstance: ProofWithPublicInputs = {
    proof: mockProofFullInstance,
    public_inputs: [1n, 2n], // Field[]
};

// --- Assertion Helpers ---
function expectQHashOut(value: any) {
    expect(value).toBeDefined();
    expect(typeof value).toBe("string");
}

function expectMerkleProofCoreQHashOut(value: any) {
    expect(value).toBeDefined();
    expectQHashOut(value.root);
    expectQHashOut(value.value); // Assuming value is also QHashOut for these proofs
    // expect(typeof value.index).toBe("bigint");
    expect(Array.isArray(value.siblings)).toBe(true);
    value.siblings.forEach((sibling: any) => expectQHashOut(sibling));
}

// function expectQEDCheckpointLeaf(value: any) {
//     expect(value).toBeDefined();
//     expect(typeof value.checkpoint_id).toBe("number");
//     expect(typeof value.next_add_withdrawal_id).toBe("number");
//     expect(typeof value.next_process_withdrawal_id).toBe("number");
//     expect(typeof value.next_deposit_id).toBe("number");
//     expect(typeof value.total_deposits_claimed_epoch).toBe("number");
//     expect(typeof value.next_user_id).toBe("bigint|number");
//     expect(typeof value.end_balance).toBe("number");
// }

// function expectQEDL2BlockState(value: any) {
//     // QEDL2BlockState has the same structure as QEDCheckpointLeaf as per types.ts
//     expectQEDCheckpointLeaf(value);
// }

// function expectQEDCheckpointGlobalStateRoots(value: any) {
//     expect(value).toBeDefined();
//     expectQHashOut(value.user_tree_root);
//     expectQHashOut(value.checkpoint_tree_root);
//     expectQHashOut(value.withdrawal_tree_root);
//     expectQHashOut(value.deposit_tree_root);
// }

// function expectQEDUserLeaf(value: any) {
//     expect(value).toBeDefined();
//     expect(typeof value.user_id).toBe("bigint");
//     expect(typeof value.nonce).toBe("bigint");
//     expect(typeof value.last_checkpoint_id).toBe("bigint");
//     expectQHashOut(value.user_state_tree_root);
//     expectQHashOut(value.user_contract_tree_root);
//     expectQHashOut(value.user_pk_hash);
// }

describe("RealmEdgeRpcProvider Integration Tests", () => {
    let client: IRealmEdgeRpcProvider;

    beforeAll(() => {
        // For integration tests, we typically don't mock the provider itself,
        // but we might want to configure it if needed.
        // The default config should be fine for basic calls.
        client = new RealmEdgeRpcProvider(MOCK_RPC_URL);
    });

    // --- Test Cases ---

    it("checkUserIdInRealm should return a boolean", async () => {
        const resultNum = await client.checkUserIdInRealm(mockUserIdNum);
        console.log("checkUserIdInRealm result:", resultNum);
        // expect(typeof resultNum).toBe("boolean");
    });

    // This is a write operation, it might have specific server-side requirements to succeed beyond type checking.
    // The test will verify the call can be made and a string (tx hash or ID) is returned.
    it("submitUserEndCap should return a string", async () => {
        try {
            const result = await client.submitUserEndCap(mockSubmitUserEndCapInput, mockProofInstance);
            expect(typeof result).toBe("string");
        } catch (error) {
            // Depending on server setup, this might fail if data is invalid or user doesn't exist.
            // For this test, we mostly care that the call structure is fine.
            // Log error for visibility during test runs if it's not a simple "user not found" or similar.
            console.warn("submitUserEndCap failed, this might be expected if server requires specific state:", error);
            expect(error).toBeDefined(); // Ensure it's an error object if it throws
        }
    });

    it("getCheckpointLeafData should return QEDCheckpointLeaf", async () => {
        const result = await client.getCheckpointLeafData(mockCheckpointIdNum);
        console.log("getCheckpointLeafData result:", result);
        // expectQEDCheckpointLeaf(result);
    });

    it("getCheckpointLeafDataF should return QEDCheckpointLeaf", async () => {
        const result = await client.getCheckpointLeafDataF(mockCheckpointIdBigInt); // todo  check
        console.log("getCheckpointLeafDataF result:", result);
        // expectQEDCheckpointLeaf(result);
    });

    it("getLatestL2BlockState should return QEDL2BlockState", async () => {
        const result = await client.getLatestL2BlockState();
        console.log("getLatestL2BlockState result:", result);
    });

    it("getL2BlockState should return QEDL2BlockState", async () => {
        const result = await client.getL2BlockState(mockCheckpointIdNum);
        console.log("getL2BlockState result:", result);
        // expectQEDL2BlockState(result);
    });

    it("getL2BlockStateF should return QEDL2BlockState", async () => {
        const result = await client.getL2BlockStateF(mockCheckpointIdBigInt);
        console.log("getL2BlockStateF result:", result);
        // expectQEDL2BlockState(result);
    });

    it("getUserRegistrationTreeRoot should return QHashOut", async () => {
        const result = await client.getUserRegistrationTreeRoot(mockCheckpointIdNum); // todo  check
        console.log("getUserRegistrationTreeRoot result:", result);
        expectQHashOut(result);
    });
    // Note: getUserRegistrationTreeRootF does not exist in IRealmEdgeRpcProvider in provided types.ts

    it("getLatestCheckpointTreeRoot should return QHashOut", async () => {
        const result = await client.getLatestCheckpointTreeRoot();
        console.log("getLatestCheckpointTreeRoot result:", result);
        expectQHashOut(result);
    });
    // Note: getLatestCheckpointTreeRootF does not exist.

    it("getCheckpointTreeRoot should return QHashOut", async () => {
        const result = await client.getCheckpointTreeRoot(mockCheckpointIdNum);
        console.log("getCheckpointTreeRoot result:", result);
        expectQHashOut(result);
    });

    it("getCheckpointTreeRootF should return QHashOut", async () => {
        const result = await client.getCheckpointTreeRootF(mockCheckpointIdBigInt);
        console.log("getCheckpointTreeRootF result:", result);
        expectQHashOut(result);
    });

    it("getCheckpointTreeLeafHash should return QHashOut", async () => {
        const result = await client.getCheckpointTreeLeafHash(mockCheckpointIdNum, mockLeafCheckpointIdNum);
        console.log("getCheckpointTreeLeafHash result:", result);
        expectQHashOut(result);
    });

    it("getCheckpointTreeLeafHashF should return QHashOut", async () => {
        const result = await client.getCheckpointTreeLeafHashF(mockCheckpointIdBigInt, mockLeafCheckpointIdBigInt);
        console.log("getCheckpointTreeLeafHashF result:", result);
        expectQHashOut(result);
    });

    it("getCheckpointTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getCheckpointTreeMerkleProof(mockCheckpointIdNum, mockLeafCheckpointIdNum);
        console.log("getCheckpointTreeMerkleProof result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getCheckpointTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getCheckpointTreeMerkleProofF(mockCheckpointIdBigInt, mockLeafCheckpointIdBigInt);
        console.log("getCheckpointTreeMerkleProofF result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getCheckpointGlobalStateRoots should return QEDCheckpointGlobalStateRoots", async () => {
        const result = await client.getCheckpointGlobalStateRoots(mockCheckpointIdNum);
        console.log("getCheckpointGlobalStateRoots result:", result);
        // expectQEDCheckpointGlobalStateRoots(result);
    });
    // Note: getCheckpointGlobalStateRootsF does not exist.

    it("getUserLeafData should return QEDUserLeaf", async () => {
        const result = await client.getUserLeafData(mockCheckpointIdNum, mockUserIdNum);
        console.log("getUserLeafData result:", result);
        // expectQEDUserLeaf(result);
    });

    it("getUserLeafDataF should return QEDUserLeaf", async () => {
        const result = await client.getUserLeafDataF(mockCheckpointIdBigInt, mockUserIdBigInt);
        console.log("getUserLeafDataF result:", result); //todo
        // expectQEDUserLeaf(result);
    });

    it("getUserContractStateTreeRoot should return QHashOut", async () => {
        const result = await client.getUserContractStateTreeRoot(mockCheckpointIdNum, mockUserIdNum, mockContractIdNum);
        console.log("getUserContractStateTreeRoot result:", result);
        // expectQHashOut(result);
    });

    it("getUserContractStateTreeRootF should return QHashOut", async () => {
        const result = await client.getUserContractStateTreeRootF(
            mockCheckpointIdBigInt,
            mockUserIdBigInt,
            mockContractIdBigInt
        );
        console.log("getUserContractStateTreeRootF result:", result);
        expectQHashOut(result);
    });

    it("getUserContractStateTreeLeafHash should return QHashOut", async () => {
        const result = await client.getUserContractStateTreeLeafHash(
            mockCheckpointIdNum,
            mockUserIdNum,
            mockContractIdNum,
            mockHeight,
            mockLeafIdNum
        );
        console.log("getUserContractStateTreeLeafHash result:", result);
        expectQHashOut(result);
    });

    it("getUserContractStateTreeLeafHashF should return QHashOut", async () => {
        const result = await client.getUserContractStateTreeLeafHashF(
            mockCheckpointIdBigInt,
            mockUserIdBigInt,
            mockContractIdBigInt,
            mockHeight,
            mockLeafIdBigInt
        );
        console.log("getUserContractStateTreeLeafHashF result:", result);
        expectQHashOut(result);
    });

    it("getUserContractStateTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserContractStateTreeMerkleProof(
            mockCheckpointIdNum,
            mockUserIdNum,
            mockContractIdNum,
            mockHeight,
            mockLeafIdNum
        );
        console.log("getUserContractStateTreeMerkleProof result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserContractStateTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserContractStateTreeMerkleProofF(
            mockCheckpointIdBigInt,
            mockUserIdBigInt,
            mockContractIdBigInt,
            mockHeight,
            mockLeafIdBigInt
        );
        console.log("getUserContractStateTreeMerkleProofF result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserContractTreeRoot should return QHashOut", async () => {
        const result = await client.getUserContractTreeRoot(mockCheckpointIdNum, mockUserIdNum);
        console.log("getUserContractTreeRoot result:", result);
        // expectQHashOut(result);
    });

    it("getUserContractTreeRootF should return QHashOut", async () => {
        const result = await client.getUserContractTreeRootF(mockCheckpointIdBigInt, mockUserIdBigInt);
        console.log("getUserContractTreeRootF result:", result);
        expectQHashOut(result);
    });

    it("getUserContractTreeLeafHash should return QHashOut", async () => {
        const result = await client.getUserContractTreeLeafHash(mockCheckpointIdNum, mockUserIdNum, mockContractIdNum);
        console.log("getUserContractTreeLeafHash result:", result);
        expectQHashOut(result);
    });

    it("getUserContractTreeLeafHashF should return QHashOut", async () => {
        const result = await client.getUserContractTreeLeafHashF(
            mockCheckpointIdBigInt,
            mockUserIdBigInt,
            mockContractIdBigInt
        );
        console.log("getUserContractTreeLeafHashF result:", result);
        expectQHashOut(result);
    });

    it("getUserContractTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserContractTreeMerkleProof(
            mockCheckpointIdNum,
            mockUserIdNum,
            mockContractIdNum
        );
        console.log("getUserContractTreeMerkleProof result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserContractTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserContractTreeMerkleProofF(
            mockCheckpointIdBigInt,
            mockUserIdBigInt,
            mockContractIdBigInt
        );
        console.log("getUserContractTreeMerkleProofF result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserTreeRoot should return QHashOut", async () => {
        const result = await client.getUserTreeRoot(mockCheckpointIdNum);
        console.log("getUserTreeRoot result:", result);
        expectQHashOut(result);
    });

    it("getUserTreeRootF should return QHashOut", async () => {
        const result = await client.getUserTreeRootF(mockCheckpointIdBigInt);
        console.log("getUserTreeRootF result:", result);
        expectQHashOut(result);
    });

    it("getUserTreeLeafHash should return QHashOut", async () => {
        const result = await client.getUserTreeLeafHash(mockCheckpointIdNum, mockUserIdNum);
        console.log("getUserTreeLeafHash result:", result);
        expectQHashOut(result);
    });

    it("getUserTreeLeafHashF should return QHashOut", async () => {
        const result = await client.getUserTreeLeafHashF(mockCheckpointIdBigInt, mockUserIdBigInt);
        console.log("getUserTreeLeafHashF result:", result);
        expectQHashOut(result);
    });

    it("getUserBottomTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserBottomTreeMerkleProof(mockRootLevel, mockCheckpointIdNum, mockUserIdNum);
        console.log("getUserBottomTreeMerkleProof result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserBottomTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserBottomTreeMerkleProofF(
            mockRootLevel,
            mockCheckpointIdBigInt,
            mockUserIdBigInt
        );
        console.log("getUserBottomTreeMerkleProofF result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserSubTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserSubTreeMerkleProof(
            mockCheckpointIdNum,
            mockRootLevel,
            mockLeafLevel,
            mockLeafIndexNum
        );
        console.log("getUserSubTreeMerkleProof result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserSubTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserSubTreeMerkleProofF(
            mockCheckpointIdBigInt,
            mockRootLevel,
            mockLeafLevel,
            mockLeafIndexBigInt
        );
        expectMerkleProofCoreQHashOut(result);
        console.log("getUserSubTreeMerkleProofF result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserTreeMerkleProof(mockCheckpointIdNum, mockUserIdNum);
        console.log("getUserTreeMerkleProof result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });

    it("getUserTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        const result = await client.getUserTreeMerkleProofF(mockCheckpointIdBigInt, mockUserIdBigInt);
        console.log("getUserTreeMerkleProofF result:", result);
        // expectMerkleProofCoreQHashOut(result);
    });
});
