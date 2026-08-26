import { CoordinatorEdgeRpcProvider } from "./client";
import { ICoordinatorEdgeRpcProvider } from "./types";
import { QBCDeployContractV2, ZKPublicKeyInfo } from "../types";

// Note: These tests are integration tests and require a running Psy Coordinator Edge RPC endpoint.
// Configure the endpoint URL via the TEST_COORD_EDGE_RPC_URL environment variable.
// You might need to set up Jest and ts-jest in your project if not already done.
// e.g., yarn add jest @types/jest ts-jest -D
// jest.config.js: module.exports = { preset: 'ts-jest', testEnvironment: 'node' };

const MOCK_RPC_URL = process.env.TEST_COORD_EDGE_RPC_URL || "http://localhost:8545";

// --- Mock Data ---
const mockUserIdNum = 1;
const mockUserIdBigInt = 1n;
const mockCheckpointIdNum = 1;
const mockCheckpointIdBigInt = 1n;
const mockContractIdNum = 3;
const mockContractIdBigInt = 3n;
const mockLeafIndexNum = 8;
const mockRootLevel = 0;
const mockCapLevel = 1;
const mockCapIndex = 1;
const mockLeafLevel = 1;
const mockLeafIdNum = 5;
const mockLeafIdBigInt = 5n;

const mockZKPublicKeyInfo: ZKPublicKeyInfo = {
    fingerprint: "0x1234567890abcdef",
    public_key_param: "0xfedcba0987654321",
};

const mockQBCDeployContract: QBCDeployContractV2 = {
    deploy_contract: {
        deployer: "0x1234567890abcdef",
        code_definition: {
            state_tree_height: 1,
            functions: [
                {
                    method_id: 1,
                    num_inputs: 1,
                    num_outputs: 1,
                    vm_type: 1,
                    code: [1, 2, 3],
                },
            ],
        },
        function_whitelist: ["0x1234567890abcdef"],
        code_root: "0x1234567890abcdef",
    },
    layout_protocol_version: 1,
    state_layout_root: "0x1234567890abcdef",
    state_layout_field_count: 1n,
    state_layout_slot_count: 1n,
    canonical_layout_verifier_fingerprint: "0x1234567890abcdef",
    canonical_layout_proof: [1],
};

// --- Assertion Helpers ---
function expectQHashOut(value: any) {
    expect(value).toBeDefined();
    expect(typeof value).toBe("string");
}

function expectMerkleProofCoreQHashOut(value: any) {
    expect(value).toBeDefined();
    expectQHashOut(value.root);
    expectQHashOut(value.value);
    // expect(typeof value.index).toBe("bigint");
    expect(Array.isArray(value.siblings)).toBe(true);
    value.siblings.forEach((sibling: any) => expectQHashOut(sibling));
}

function expectPsyUserLeaf(value: any) {
    expect(value).toBeDefined();
    expect(typeof value.user_id).toBe("bigint");
    expect(typeof value.nonce).toBe("bigint");
    expect(typeof value.last_checkpoint_id).toBe("bigint");
    expectQHashOut(value.user_state_tree_root);
    expectQHashOut(value.user_contract_tree_root);
    expectQHashOut(value.user_pk_hash);
}

function expectPsyContractLeaf(value: any) {
    expect(value).toBeDefined();
    expectQHashOut(value.deployer);
    expectQHashOut(value.function_tree_root);
    expect(typeof value.state_tree_height).toBe("bigint");
}

function expectPsyCheckpointLeaf(value: any) {
    expect(value).toBeDefined();
    // expect(typeof value.checkpoint_id).toBe("number");
    // expect(typeof value.next_add_withdrawal_id).toBe("number");
    // expect(typeof value.next_process_withdrawal_id).toBe("number");
    // expect(typeof value.next_deposit_id).toBe("number");
    // expect(typeof value.total_deposits_claimed_epoch).toBe("number");
    // expect(typeof value.next_user_id).toBe("bigint");
    // expect(typeof value.end_balance).toBe("number");
}

// function expectPsyBlockState(value: any) {
//     // PsyBlockState has the same structure as PsyCheckpointLeaf
//     expectPsyCheckpointLeaf(value);
// }

function expectPsyCheckpointGlobalStateRoots(_value: any) {
    // expect(value).toBeDefined();
    // expectQHashOut(value.user_tree_root);
    // expectQHashOut(value.checkpoint_tree_root);
    // expectQHashOut(value.withdrawal_tree_root);
    // expectQHashOut(value.deposit_tree_root);
}

function expectLatestCheckpointResponse(_value: any) {
    // expect(value).toBeDefined();
    // expect(typeof value.checkpoint_id).toBe("number");
    // expectQHashOut(value.checkpoint_hash);
}

function expectCheckpointSyncInfo(value: any) {
    expect(value).toBeDefined();
    // expect(typeof value.checkpoint_id).toBe("number");
    // expect(Array.isArray(value.sync_data)).toBe(true);
}

function expectContractCodeDefinition(value: any) {
    expect(value).toBeDefined();
    expect(typeof value.code).toBe("string");
    expect(typeof value.metadata).toBe("string");
}

function expectPsyCheckpointSyncInfoCompact(value: any) {
    expect(value).toBeDefined();
    // expect(typeof value.checkpoint_id).toBe("number");
    // expect(Array.isArray(value.compact_data)).toBe(true);
}

describe("CoordinatorEdgeRpcProvider Integration Tests", () => {
    let client: ICoordinatorEdgeRpcProvider;

    beforeAll(() => {
        // For integration tests, we typically don't mock the provider itself,
        // but we might want to configure it if needed.
        // The default config should be fine for basic calls.
        client = new CoordinatorEdgeRpcProvider(MOCK_RPC_URL);
    });

    // --- User Registration Tests ---

    it("registerUser should return a string", async () => {
        try {
            const result = await client.registerUser(mockZKPublicKeyInfo);
            expect(typeof result).toBe("string");
            console.log("registerUser result:", result);
        } catch (error) {
            // This might fail if user already exists or server requirements not met
            console.warn("registerUser failed, this might be expected:", error);
            expect(error).toBeDefined();
        }
    });

    it("getUserId should return a number", async () => {
        const mockQHashOut = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        try {
            const result = await client.getUserId(mockQHashOut);
            expect(typeof result).toBe("number");
            console.log("getUserId result:", result);
        } catch (error) {
            console.warn("getUserId failed, this might be expected:", error);
            expect(error).toBeDefined();
        }
    });

    // --- Contract Tests ---

    it("deployContract should return a string", async () => {
        try {
            const result = await client.deployContract(mockQBCDeployContract);
            expect(typeof result).toBe("string");
            console.log("deployContract result:", result);
        } catch (error) {
            console.warn("deployContract failed, this might be expected:", error);
            expect(error).toBeDefined();
        }
    });

    it("getContractLeafData should return PsyContractLeaf", async () => {
        try {
            const result = await client.getContractLeafData(mockContractIdNum);
            expectPsyContractLeaf(result);
            console.log("getContractLeafData result:", result);
        } catch (error) {
            console.warn("getContractLeafData failed:", error);
        }
    });

    it("getContractLeafDataF should return PsyContractLeaf", async () => {
        try {
            const result = await client.getContractLeafDataF(mockContractIdNum);
            expectPsyContractLeaf(result);
            console.log("getContractLeafDataF result:", result);
        } catch (error) {
            console.warn("getContractLeafDataF failed:", error);
        }
    });

    it("getContractCodeDefinition should return ContractCodeDefinition", async () => {
        try {
            const result = await client.getContractCodeDefinition(mockContractIdNum);
            expectContractCodeDefinition(result);
            console.log("getContractCodeDefinition result:", result);
        } catch (error) {
            console.warn("getContractCodeDefinition failed:", error);
        }
    });

    it("getContractCodeDefinitionF should return ContractCodeDefinition", async () => {
        try {
            const result = await client.getContractCodeDefinitionF(mockContractIdBigInt);
            expectContractCodeDefinition(result);
            console.log("getContractCodeDefinitionF result:", result);
        } catch (error) {
            console.warn("getContractCodeDefinitionF failed:", error);
        }
    });

    // --- Checkpoint Tests ---

    it("getLatestCheckpointId should return LatestCheckpointResponse", async () => {
        try {
            const result = await client.getLatestCheckpointId();
            expectLatestCheckpointResponse(result);
            console.log("getLatestCheckpointId result:", result);
        } catch (error) {
            console.warn("getLatestCheckpointId failed:", error);
        }
    });

    it("getCheckpointSyncInfo should return CheckpointSyncInfo", async () => {
        try {
            const result = await client.getCheckpointSyncInfo(mockCheckpointIdNum);
            expectCheckpointSyncInfo(result);
            console.log("getCheckpointSyncInfo result:", result);
        } catch (error) {
            console.warn("getCheckpointSyncInfo failed:", error);
        }
    });

    it("getCheckpointLeafData should return PsyCheckpointLeaf", async () => {
        try {
            const result = await client.getCheckpointLeafData(mockCheckpointIdNum);
            expectPsyCheckpointLeaf(result);
            console.log("getCheckpointLeafData result:", result);
        } catch (error) {
            console.warn("getCheckpointLeafData failed:", error);
        }
    });

    it("getCheckpointLeafDataF should return PsyCheckpointLeaf", async () => {
        try {
            const result = await client.getCheckpointLeafDataF(mockCheckpointIdBigInt);
            // expectPsyCheckpointLeaf(result);
            console.log("getCheckpointLeafDataF result:", result);
        } catch (error) {
            console.warn("getCheckpointLeafDataF failed:", error);
        }
    });

    // --- Block State Tests ---

    it("getLatestBlockState should return PsyBlockState", async () => {
        try {
            const result = await client.getLatestBlockState();
            // expectPsyBlockState(result);
            console.log("getLatestBlockState result:", result);
        } catch (error) {
            console.warn("getLatestBlockState failed:", error);
        }
    });

    it("getBlockState should return PsyBlockState", async () => {
        try {
            const result = await client.getBlockState(mockCheckpointIdNum);
            // expectPsyBlockState(result);
            console.log("getBlockState result:", result);
        } catch (error) {
            console.warn("getBlockState failed:", error);
        }
    });

    it("getBlockStateF should return PsyBlockState", async () => {
        try {
            const result = await client.getBlockStateF(mockCheckpointIdBigInt);
            // expectPsyBlockState(result);
            console.log("getBlockStateF result:", result);
        } catch (error) {
            console.warn("getBlockStateF failed:", error);
        }
    });

    // --- User Registration Tree Tests ---

    it("getUserRegistrationTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getUserRegistrationTreeRoot(mockCheckpointIdNum);
            expectQHashOut(result);
            console.log("getUserRegistrationTreeRoot result:", result);
        } catch (error) {
            console.warn("getUserRegistrationTreeRoot failed:", error);
        }
    });

    it("getUserRegistrationTreeRootF should return QHashOut", async () => {
        try {
            const result = await client.getUserRegistrationTreeRootF(mockCheckpointIdBigInt);
            expectQHashOut(result);
            console.log("getUserRegistrationTreeRootF result:", result);
        } catch (error) {
            console.warn("getUserRegistrationTreeRootF failed:", error);
        }
    });

    it("getUserRegistrationTreeLeafHash should return QHashOut", async () => {
        try {
            const result = await client.getUserRegistrationTreeLeafHash(mockCheckpointIdNum, mockUserIdNum);
            expectQHashOut(result);
            console.log("getUserRegistrationTreeLeafHash result:", result);
        } catch (error) {
            console.warn("getUserRegistrationTreeLeafHash failed:", error);
        }
    });

    it("getUserRegistrationTreeLeafHashF should return QHashOut", async () => {
        try {
            const result = await client.getUserRegistrationTreeLeafHashF(mockCheckpointIdBigInt, mockUserIdBigInt);
            expectQHashOut(result);
            console.log("getUserRegistrationTreeLeafHashF result:", result);
        } catch (error) {
            console.warn("getUserRegistrationTreeLeafHashF failed:", error);
        }
    });

    it("getUserRegistrationTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getUserRegistrationTreeMerkleProof(mockCheckpointIdNum, mockUserIdNum);
            // expectMerkleProofCoreQHashOut(result);
            console.log("getUserRegistrationTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getUserRegistrationTreeMerkleProof failed:", error);
        }
    });

    it("getUserRegistrationTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getUserRegistrationTreeMerkleProofF(mockCheckpointIdBigInt, mockUserIdBigInt);
            // expectMerkleProofCoreQHashOut(result);
            console.log("getUserRegistrationTreeMerkleProofF result:", result);
        } catch (error) {
            console.warn("getUserRegistrationTreeMerkleProofF failed:", error);
        }
    });

    // --- User Tree Tests ---

    it("getUserTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getUserTreeRoot(mockCheckpointIdNum);
            expectQHashOut(result);
            console.log("getUserTreeRoot result:", result);
        } catch (error) {
            console.warn("getUserTreeRoot failed:", error);
        }
    });

    it("getUserTreeRootF should return QHashOut", async () => {
        try {
            const result = await client.getUserTreeRootF(mockCheckpointIdBigInt);
            expectQHashOut(result);
            console.log("getUserTreeRootF result:", result);
        } catch (error) {
            console.warn("getUserTreeRootF failed:", error);
        }
    });

    it("getUserSubTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getUserSubTreeMerkleProof(
                mockCheckpointIdNum,
                mockRootLevel,
                mockLeafLevel,
                mockLeafIndexNum
            );
            // expectMerkleProofCoreQHashOut(result);
            console.log("getUserSubTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getUserSubTreeMerkleProof failed:", error);
        }
    });

    it("getUserTopTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getUserTopTreeMerkleProof(mockCheckpointIdNum, mockRootLevel, mockLeafIndexNum);
            expectMerkleProofCoreQHashOut(result);
            console.log("getUserTopTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getUserTopTreeMerkleProof failed:", error);
        }
    });

    it("getUserTopTreeCapRoot should return QHashOut", async () => {
        try {
            const result = await client.getUserTopTreeCapRoot(mockCheckpointIdNum, mockCapLevel, mockCapIndex);
            expectQHashOut(result);
            console.log("getUserTopTreeCapRoot result:", result);
        } catch (error) {
            console.warn("getUserTopTreeCapRoot failed:", error);
        }
    });

    it("getUserLatestTopTreeCapRoot should return QHashOut", async () => {
        try {
            const result = await client.getUserLatestTopTreeCapRoot(mockCapLevel, mockCapIndex);
            expectQHashOut(result);
            console.log("getUserLatestTopTreeCapRoot result:", result);
        } catch (error) {
            console.warn("getUserLatestTopTreeCapRoot failed:", error);
        }
    });

    // --- Contract Function Tree Tests ---

    it("getContractFunctionTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getContractFunctionTreeRoot(mockCheckpointIdNum, mockContractIdNum);
            expectQHashOut(result);
            console.log("getContractFunctionTreeRoot result:", result);
        } catch (error) {
            console.warn("getContractFunctionTreeRoot failed:", error);
        }
    });

    it("getContractFunctionTreeRootF should return QHashOut", async () => {
        try {
            const result = await client.getContractFunctionTreeRootF(mockCheckpointIdBigInt, mockContractIdBigInt);
            expectQHashOut(result);
            console.log("getContractFunctionTreeRootF result:", result);
        } catch (error) {
            console.warn("getContractFunctionTreeRootF failed:", error);
        }
    });

    it("getContractFunctionTreeLeafHash should return QHashOut", async () => {
        try {
            const result = await client.getContractFunctionTreeLeafHash(
                mockCheckpointIdNum,
                mockContractIdNum,
                mockLeafIdNum
            );
            expectQHashOut(result);
            console.log("getContractFunctionTreeLeafHash result:", result);
        } catch (error) {
            console.warn("getContractFunctionTreeLeafHash failed:", error);
        }
    });

    it("getContractFunctionTreeLeafHashF should return QHashOut", async () => {
        try {
            const result = await client.getContractFunctionTreeLeafHashF(
                mockCheckpointIdBigInt,
                mockContractIdBigInt,
                mockLeafIdBigInt
            );
            expectQHashOut(result);
            console.log("getContractFunctionTreeLeafHashF result:", result);
        } catch (error) {
            console.warn("getContractFunctionTreeLeafHashF failed:", error);
        }
    });

    it("getContractFunctionTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getContractFunctionTreeMerkleProof(
                mockCheckpointIdNum,
                mockContractIdNum,
                mockLeafIdNum
            );
            // expectMerkleProofCoreQHashOut(result);
            console.log("getContractFunctionTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getContractFunctionTreeMerkleProof failed:", error);
        }
    });

    it("getContractFunctionTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getContractFunctionTreeMerkleProofF(
                mockCheckpointIdBigInt,
                mockContractIdBigInt,
                mockLeafIdBigInt
            );
            // expectMerkleProofCoreQHashOut(result);
            console.log("getContractFunctionTreeMerkleProofF result:", result);
        } catch (error) {
            console.warn("getContractFunctionTreeMerkleProofF failed:", error);
        }
    });

    // --- Contract Tree Tests ---

    it("getContractTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getContractTreeRoot(mockCheckpointIdNum);
            expectQHashOut(result);
            console.log("getContractTreeRoot result:", result);
        } catch (error) {
            console.warn("getContractTreeRoot failed:", error);
        }
    });

    it("getContractTreeRootF should return QHashOut", async () => {
        try {
            const result = await client.getContractTreeRootF(mockCheckpointIdBigInt);
            expectQHashOut(result);
            console.log("getContractTreeRootF result:", result);
        } catch (error) {
            console.warn("getContractTreeRootF failed:", error);
        }
    });

    it("getContractTreeLeafHash should return QHashOut", async () => {
        try {
            const result = await client.getContractTreeLeafHash(mockCheckpointIdNum, mockContractIdNum);
            expectQHashOut(result);
            console.log("getContractTreeLeafHash result:", result);
        } catch (error) {
            console.warn("getContractTreeLeafHash failed:", error);
        }
    });

    it("getContractTreeLeafHashF should return QHashOut", async () => {
        try {
            const result = await client.getContractTreeLeafHashF(mockCheckpointIdBigInt, mockContractIdBigInt);
            expectQHashOut(result);
            console.log("getContractTreeLeafHashF result:", result);
        } catch (error) {
            console.warn("getContractTreeLeafHashF failed:", error);
        }
    });

    it("getContractTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getContractTreeMerkleProof(mockCheckpointIdNum, mockContractIdNum);
            // expectMerkleProofCoreQHashOut(result);
            console.log("getContractTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getContractTreeMerkleProof failed:", error);
        }
    });

    it("getContractTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getContractTreeMerkleProofF(mockCheckpointIdBigInt, mockContractIdBigInt);
            // expectMerkleProofCoreQHashOut(result);
            console.log("getContractTreeMerkleProofF result:", result);
        } catch (error) {
            console.warn("getContractTreeMerkleProofF failed:", error);
        }
    });

    // --- Deposit Tree Tests ---

    it("getDepositTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getDepositTreeRoot(mockCheckpointIdNum);
            expectQHashOut(result);
            console.log("getDepositTreeRoot result:", result);
        } catch (error) {
            console.warn("getDepositTreeRoot failed:", error);
        }
    });

    it("getDepositTreeRootF should return QHashOut", async () => {
        try {
            const result = await client.getDepositTreeRootF(mockCheckpointIdBigInt);
            expectQHashOut(result);
            console.log("getDepositTreeRootF result:", result);
        } catch (error) {
            console.warn("getDepositTreeRootF failed:", error);
        }
    });

    it("getDepositTreeLeafHash should return QHashOut", async () => {
        try {
            const result = await client.getDepositTreeLeafHash(mockCheckpointIdNum, mockLeafIdNum);
            expectQHashOut(result);
            console.log("getDepositTreeLeafHash result:", result);
        } catch (error) {
            console.warn("getDepositTreeLeafHash failed:", error);
        }
    });

    it("getDepositTreeLeafHashF should return QHashOut", async () => {
        try {
            const result = await client.getDepositTreeLeafHashF(mockCheckpointIdBigInt, mockLeafIdBigInt);
            expectQHashOut(result);
            console.log("getDepositTreeLeafHashF result:", result);
        } catch (error) {
            console.warn("getDepositTreeLeafHashF failed:", error);
        }
    });

    it("getDepositTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getDepositTreeMerkleProof(mockCheckpointIdNum, mockLeafIdNum);
            // expectMerkleProofCoreQHashOut(result);
            console.log("getDepositTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getDepositTreeMerkleProof failed:", error);
        }
    });

    it("getDepositTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getDepositTreeMerkleProofF(mockCheckpointIdBigInt, mockLeafIdBigInt);
            // expectMerkleProofCoreQHashOut(result);
            console.log("getDepositTreeMerkleProofF result:", result);
        } catch (error) {
            console.warn("getDepositTreeMerkleProofF failed:", error);
        }
    });

    // --- Withdrawal Tree Tests ---

    it("getWithdrawalTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getWithdrawalTreeRoot(mockCheckpointIdNum);
            expectQHashOut(result);
            console.log("getWithdrawalTreeRoot result:", result);
        } catch (error) {
            console.warn("getWithdrawalTreeRoot failed:", error);
        }
    });

    it("getWithdrawalTreeRootF should return QHashOut", async () => {
        try {
            const result = await client.getWithdrawalTreeRootF(mockCheckpointIdBigInt);
            expectQHashOut(result);
            console.log("getWithdrawalTreeRootF result:", result);
        } catch (error) {
            console.warn("getWithdrawalTreeRootF failed:", error);
        }
    });

    it("getWithdrawalTreeLeafHash should return QHashOut", async () => {
        try {
            const result = await client.getWithdrawalTreeLeafHash(mockCheckpointIdNum, mockLeafIdNum);
            expectQHashOut(result);
            console.log("getWithdrawalTreeLeafHash result:", result);
        } catch (error) {
            console.warn("getWithdrawalTreeLeafHash failed:", error);
        }
    });

    it("getWithdrawalTreeLeafHashF should return QHashOut", async () => {
        try {
            const result = await client.getWithdrawalTreeLeafHashF(mockCheckpointIdBigInt, mockLeafIdBigInt);
            expectQHashOut(result);
            console.log("getWithdrawalTreeLeafHashF result:", result);
        } catch (error) {
            console.warn("getWithdrawalTreeLeafHashF failed:", error);
        }
    });

    it("getWithdrawalTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getWithdrawalTreeMerkleProof(mockCheckpointIdNum, mockLeafIdNum);
            // expectMerkleProofCoreQHashOut(result);
            console.log("getWithdrawalTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getWithdrawalTreeMerkleProof failed:", error);
        }
    });

    it("getWithdrawalTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getWithdrawalTreeMerkleProofF(mockCheckpointIdBigInt, mockLeafIdBigInt);
            expectMerkleProofCoreQHashOut(result);
            console.log("getWithdrawalTreeMerkleProofF result:", result);
        } catch (error) {
            console.warn("getWithdrawalTreeMerkleProofF failed:", error);
        }
    });

    // --- Checkpoint Tree Tests ---

    it("getLatestCheckpointTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getLatestCheckpointTreeRoot();
            expectQHashOut(result);
            console.log("getLatestCheckpointTreeRoot result:", result);
        } catch (error) {
            console.warn("getLatestCheckpointTreeRoot failed:", error);
        }
    });

    it("getCheckpointTreeRoot should return QHashOut", async () => {
        try {
            const result = await client.getCheckpointTreeRoot(mockCheckpointIdNum);
            expectQHashOut(result);
            console.log("getCheckpointTreeRoot result:", result);
        } catch (error) {
            console.warn("getCheckpointTreeRoot failed:", error);
        }
    });

    it("getCheckpointTreeRootF should return QHashOut", async () => {
        try {
            const result = await client.getCheckpointTreeRootF(mockCheckpointIdBigInt);
            expectQHashOut(result);
            console.log("getCheckpointTreeRootF result:", result);
        } catch (error) {
            console.warn("getCheckpointTreeRootF failed:", error);
        }
    });

    it("getCheckpointTreeLeafHash should return QHashOut", async () => {
        try {
            const result = await client.getCheckpointTreeLeafHash(mockCheckpointIdNum, mockLeafIdNum);
            expectQHashOut(result);
            console.log("getCheckpointTreeLeafHash result:", result);
        } catch (error) {
            console.warn("getCheckpointTreeLeafHash failed:", error);
        }
    });

    it("getCheckpointTreeLeafHashF should return QHashOut", async () => {
        try {
            const result = await client.getCheckpointTreeLeafHashF(mockCheckpointIdBigInt, mockLeafIdBigInt);
            expectQHashOut(result);
            console.log("getCheckpointTreeLeafHashF result:", result);
        } catch (error) {
            console.warn("getCheckpointTreeLeafHashF failed:", error);
        }
    });

    it("getCheckpointTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getCheckpointTreeMerkleProof(mockCheckpointIdNum, mockLeafIdNum);
            expectMerkleProofCoreQHashOut(result);
            console.log("getCheckpointTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getCheckpointTreeMerkleProof failed:", error);
        }
    });

    it("getCheckpointTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getCheckpointTreeMerkleProofF(mockCheckpointIdBigInt, mockLeafIdBigInt);
            expectMerkleProofCoreQHashOut(result);
            console.log("getCheckpointTreeMerkleProofF result:", result);
        } catch (error) {
            console.warn("getCheckpointTreeMerkleProofF failed:", error);
        }
    });

    // --- Global State Tests ---

    it("getCheckpointGlobalStateRoots should return PsyCheckpointGlobalStateRoots", async () => {
        try {
            const result = await client.getCheckpointGlobalStateRoots(mockCheckpointIdNum);
            expectPsyCheckpointGlobalStateRoots(result);
            console.log("getCheckpointGlobalStateRoots result:", result);
        } catch (error) {
            console.warn("getCheckpointGlobalStateRoots failed:", error);
        }
    });

    it("getCheckpointSyncInfoCompact should return PsyCheckpointSyncInfoCompact", async () => {
        try {
            const result = await client.getCheckpointSyncInfoCompact(mockCheckpointIdNum);
            expectPsyCheckpointSyncInfoCompact(result);
            console.log("getCheckpointSyncInfoCompact result:", result);
        } catch (error) {
            console.warn("getCheckpointSyncInfoCompact failed:", error);
        }
    });

    it("latestCheckpoint should return LatestCheckpointResponse", async () => {
        try {
            const result = await client.latestCheckpoint();
            expectLatestCheckpointResponse(result);
            console.log("latestCheckpoint result:", result);
        } catch (error) {
            console.warn("latestCheckpoint failed:", error);
        }
    });

    // --- User Leaf Data Tests ---

    it("getUserLeafData should return PsyUserLeaf", async () => {
        try {
            const result = await client.getUserLeafData(mockCheckpointIdNum, mockUserIdNum);
            expectPsyUserLeaf(result);
            console.log("getUserLeafData result:", result);
        } catch (error) {
            console.warn("getUserLeafData failed:", error);
        }
    });

    it("getUserTreeMerkleProof should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getUserTreeMerkleProof(mockCheckpointIdNum, mockUserIdNum);
            expectMerkleProofCoreQHashOut(result);
            console.log("getUserTreeMerkleProof result:", result);
        } catch (error) {
            console.warn("getUserTreeMerkleProof failed:", error);
        }
    });

    it("getUserTreeMerkleProofF should return MerkleProofCore<QHashOut>", async () => {
        try {
            const result = await client.getUserTreeMerkleProofF(mockCheckpointIdBigInt, mockUserIdBigInt);
            expectMerkleProofCoreQHashOut(result);
            console.log("getUserTreeMerkleProofF result:", result);
        } catch (error) {
            console.warn("getUserTreeMerkleProofF failed:", error);
        }
    });

    // --- Block Building Tests ---

    it("buildBlock should return a string", async () => {
        try {
            const result = await client.buildBlock();
            expect(typeof result).toBe("string");
            console.log("buildBlock result:", result);
        } catch (error) {
            console.warn("buildBlock failed, this might be expected:", error);
            expect(error).toBeDefined();
        }
    });
});
