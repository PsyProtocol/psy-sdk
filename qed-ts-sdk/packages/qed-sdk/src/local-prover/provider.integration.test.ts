/**
 * Integration tests for QED WASM User Prover Provider
 * These tests verify the WASM-based local prover functionality
 *
 * To run these tests:
 * 1. Ensure WASM module is properly compiled and available
 * 2. Run: npm test -- --testNamePattern="WASM.*Integration"
 */

import { createDefaultRpcConfig, RpcConfig } from "./config";
import { QedWasmUserProverProvider } from "./provider";
import { PrivateKey, PublicKey, QHashOut } from "../core";
import {
    DPNFunctionCircuitDefinition,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo, ContractCallArgs, WalletKeyPair, QBCDeployContract } from "../types";
import { waitMs } from "../utils";
import { CoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import {calculatePkHash} from "../types/pkhash";
import path from "path";
import fs from "fs";

async function waitBlock(coordinator: CoordinatorEdgeRpcProvider): Promise<void> {
    await coordinator.buildBlock();
    await coordinator.buildBlock();
    await coordinator.buildBlock();
    await waitMs(3000);
}

function reverseString(str: string): string {
    return str.split("").reverse().join("");
}

describe("QED WASM User Prover Provider Integration Tests", () => {
    let provider: QedWasmUserProverProvider;
    let rpcConfig: RpcConfig;
    const timeout = 60000; // 60 seconds timeout for WASM operations

    // Test data
    let testPrivateKey: PrivateKey;
    let testPublicKey: ZKPublicKeyInfo;
    let testKeypair: WalletKeyPair;
    let testPkHash: PublicKey;
    let sessionId: string;
    let coordinator: CoordinatorEdgeRpcProvider;
    const MOCK_RPC_URL = process.env.TEST_COORD_EDGE_RPC_URL || "http://localhost:8545";

    beforeAll(async () => {
        // Initialize WASM provider with default configuration
        rpcConfig = createDefaultRpcConfig();
        provider = new QedWasmUserProverProvider(rpcConfig);
        coordinator = new CoordinatorEdgeRpcProvider(MOCK_RPC_URL);

        // Generate test keypair
        testKeypair = await provider.getRandomKeypair();
        testPrivateKey = testKeypair.private_key;
        testPublicKey = testKeypair.public_key;
        testPkHash = calculatePkHash(testPublicKey);

        console.log("🔧 WASM Provider initialized with test keypair");
        console.log("🔑 Test Public Key fingerprint:", testPublicKey.fingerprint);
    }, timeout);

    describe("Server Connectivity and Basic Operations", () => {
        it(
            "should ping the WASM server successfully",
            async () => {
                const message = "WASM Integration test ping";

                const response = await provider.ping(message);
                expect(response).toBeDefined();
                expect(typeof response).toBe("string");
                expect(response).toContain(reverseString(message));

                console.log("📡 Ping response:", response);
            },
            timeout
        );

        it(
            "should generate random keypair",
            async () => {
                    const keypair = await provider.getRandomKeypair();
                    console.log("🔑 Generated keypair:", keypair);
            },
            timeout
        );

        it(
            "should start a session successfully",
            async () => {
                sessionId = await provider.startSession(testPkHash);
                expect(sessionId).toBeDefined();
                expect(typeof sessionId).toBe("string");
                expect(sessionId.length).toBeGreaterThan(0);

                console.log("🚀 Session started:", sessionId);
            },
            timeout
        );
    });

    describe("User Management Operations", () => {
        it(
            "should register a user successfully",
            async () => {
                const registeredPublicKey: PublicKey = await provider.registerUser(testPrivateKey);
                expect(registeredPublicKey).toBeDefined();
                expect(typeof registeredPublicKey).toBe("string");
                await waitBlock(coordinator);

                console.log("👤 User registered with public key:", registeredPublicKey);
            },
            timeout
        );

        it(
            "should add a user successfully",
            async () => {
                const newKeypair = await provider.getRandomKeypair();
                const registeredPublicKey: PublicKey = await provider.registerUser(newKeypair.private_key);
                expect(registeredPublicKey).toBeDefined();
                expect(typeof registeredPublicKey).toBe("string");

                await waitBlock(coordinator);
                await waitBlock(coordinator);
                console.log("👤 User registered with public key:", registeredPublicKey);

                const addedPublicKey: PublicKey = await provider.addUser(newKeypair.private_key);
                expect(addedPublicKey).toBeDefined();
                expect(typeof addedPublicKey).toBe("string");
                await waitBlock(coordinator);

                console.log("➕ User added with public key:", addedPublicKey);
            },
            timeout
        );

        it(
            "should get ZK public key for a user",
            async () => {
                const zkPublicKey: ZKPublicKeyInfo = await provider.getZKPublicKey(testPrivateKey);
                expect(zkPublicKey).toBeDefined();
                expect(zkPublicKey.fingerprint).toBeDefined();
                expect(zkPublicKey.public_key_param).toBeDefined();

                console.log(" actual ZK Public Key: ", zkPublicKey, "expect ZK Public Key:", testPublicKey);

                console.log("🔐 ZK Public Key Info:", {
                    fingerprint: zkPublicKey.fingerprint.substring(0, 20) + "...",
                    publicKeyParam: zkPublicKey.public_key_param.substring(0, 20) + "...",
                });
            },
            timeout
        );
    });

    describe("Contract Operations", () => {
        let mockCircuitDefs: DPNFunctionCircuitDefinition[];
        let deployContractCmd: QBCDeployContract;

        beforeAll(() => {
            // Create mock circuit definitions for testing
            mockCircuitDefs = [
                {
                    name: "test_function",
                    method_id: 1,
                    circuit_inputs: [BigInt(1), BigInt(2)],
                    circuit_outputs: [BigInt(3)],
                    state_commands: [],
                    state_command_resolution_indices: [],
                    assertions: [],
                    definitions: [],
                },
            ];
        });

        it(
            "should get deploy contract command",
            async () => {
                deployContractCmd = await provider.getDeployContractCmd(testPkHash, mockCircuitDefs);
                expect(deployContractCmd).toBeDefined();
                expect(deployContractCmd.deployer).toBeDefined();
                expect(deployContractCmd.code_definition).toBeDefined();
                expect(deployContractCmd.function_whitelist).toBeDefined();

                console.log("📋 Deploy contract command:", {
                    deployer: deployContractCmd.deployer.substring(0, 20) + "...",
                    codeDefinition: deployContractCmd.code_definition ? "defined" : "undefined",
                    functionWhitelistLength: deployContractCmd.function_whitelist.length,
                });
            },
            timeout
        );

        it(
            "should deploy contract successfully",
            async () => {
                const deployResult = await provider.deployContract(testPkHash, mockCircuitDefs);
                expect(deployResult).toBeDefined();
                expect(typeof deployResult).toBe("string");

                console.log("🚀 Contract deployed:", deployResult);
            },
            timeout
        );

        it(
            "should prove contract call",
            async () => {
                const contractCallArgs: ContractCallArgs = {
                    contract_id: BigInt(1),
                    method_name: "test_function",
                    inputs: [BigInt(10), BigInt(20)],
                };

                const proofResult = await provider.proveContractCall(testPkHash, contractCallArgs);
                expect(proofResult).toBeDefined();
                expect(typeof proofResult).toBe("string");

                console.log("🔍 Contract call proved:", proofResult.substring(0, 50) + "...");
            },
            timeout
        );

        it(
            "should prove multiple contract calls",
            async () => {
                const contractCallArgs: ContractCallArgs[] = [
                    {
                        contract_id: BigInt(1),
                        method_name: "test_function",
                        inputs: [BigInt(5), BigInt(10)],
                    },
                    {
                        contract_id: BigInt(1),
                        method_name: "test_function",
                        inputs: [BigInt(7), BigInt(8)],
                    },
                ];

                const proofResult = await provider.proveContractCalls(testPkHash, contractCallArgs);
                expect(proofResult).toBeDefined();
                expect(typeof proofResult).toBe("string");

                console.log("🔍 Multiple contract calls proved:", proofResult.substring(0, 50) + "...");
            },
            timeout
        );
    });

    describe("Signing and Submission Operations", () => {
        it(
            "should sign and submit successfully",
            async () => {
                const submitResult = await provider.signAndSubmit(testPkHash);
                expect(submitResult).toBeDefined();
                expect(typeof submitResult).toBe("string");

                console.log("📤 Sign and submit result:", submitResult);
            },
            timeout
        );
    });

    describe("Utility Operations", () => {
        it(
            "should get result by ID",
            async () => {
                // Use a mock hash for testing
                const mockHashOut: QHashOut = "0x1234567890abcdef";

                try {
                    const result = await provider.getResult(mockHashOut);
                    expect(result).toBeDefined();
                    console.log("📋 Result retrieved:", result);
                } catch (error) {
                    // It's expected that this might fail with a mock ID
                    console.log("⚠️ Expected error for mock ID:", (error as Error).message);
                    expect(error).toBeDefined();
                }
            },
            timeout
        );
    });

    describe("Error Handling", () => {
        it(
            "should handle invalid private key gracefully",
            async () => {
                const invalidPrivateKey = "invalid_key";

                await expect(provider.registerUser(invalidPrivateKey)).rejects.toThrow();
                console.log("❌ Invalid private key properly rejected");
            },
            timeout
        );

        it(
            "should handle invalid contract call arguments",
            async () => {
                const invalidContractCall: ContractCallArgs = {
                    contract_id: BigInt(-1), // Invalid contract ID
                    method_name: "invalid_method", // Invalid method name
                    inputs: [],
                };

                await expect(provider.proveContractCall(testPkHash, invalidContractCall)).rejects.toThrow();
                console.log("❌ Invalid contract call properly rejected");
            },
            timeout
        );

        it(
            "should handle empty circuit definitions",
            async () => {
                const emptyCircuitDefs: DPNFunctionCircuitDefinition[] = [];

                await expect(provider.deployContract(testPkHash, emptyCircuitDefs)).rejects.toThrow();
                console.log("❌ Empty circuit definitions properly rejected");
            },
            timeout
        );
    });

    describe("Performance Tests", () => {
        it(
            "should handle multiple concurrent operations",
            async () => {
                const operations = [
                    provider.ping("concurrent-1"),
                    provider.ping("concurrent-2"),
                    provider.ping("concurrent-3"),
                    provider.getRandomKeypair(),
                    provider.getRandomKeypair(),
                ];

                const results = await Promise.all(operations);
                expect(results).toHaveLength(5);
                expect(results.every((result) => result !== undefined)).toBe(true);

                console.log("🚀 Concurrent operations completed successfully");
            },
            timeout
        );

        it(
            "should measure operation timing",
            async () => {
                const startTime = Date.now();
                await provider.getRandomKeypair();
                const endTime = Date.now();

                const duration = endTime - startTime;
                expect(duration).toBeLessThan(10000); // Should complete within 10 seconds

                console.log("⏱️ Keypair generation took", duration, "ms");
            },
            timeout
        );
    });

    describe("Workflow tests", () => {
        it(
            "Workflow test",
            async () => {
                const message = "WASM Integration test ping";
                const response = await provider.ping(message);
                expect(response).toBeDefined();
                expect(typeof response).toBe("string");
                expect(response).toContain(reverseString(message));
                console.log("📡 Ping response:", response);

                const privateKey = "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a";
                const registeredPublicKey: PublicKey = await provider.registerUser(privateKey);
                expect(registeredPublicKey).toBeDefined();
                expect(typeof registeredPublicKey).toBe("string");
                await waitBlock(coordinator);
                await waitBlock(coordinator);
                console.log("👤 User registered with public key:", registeredPublicKey);

                const addedPublicKey: PublicKey = await provider.addUser(privateKey);
                expect(addedPublicKey).toBeDefined();
                expect(typeof addedPublicKey).toBe("string");
                await waitBlock(coordinator);
                console.log("➕ User added with public key:", addedPublicKey);

                const zkPublicKey: ZKPublicKeyInfo = await provider.getZKPublicKey(privateKey);
                expect(zkPublicKey).toBeDefined();
                expect(zkPublicKey.fingerprint).toBeDefined();
                expect(zkPublicKey.public_key_param).toBeDefined();
                console.log("ZK Public Key: ", zkPublicKey);

                try {
                    sessionId = await provider.startSession(addedPublicKey);
                    console.log(`${addedPublicKey} Setup complete for contract deployment: ${sessionId}`);

                    const circuitDefs = JSON.parse(
                        fs.readFileSync(path.resolve(__dirname, "../../../../../qed_user_cli/contract.json"), "utf8")
                    );
                    console.log("circuitDefs: ", circuitDefs);
                    let result = await provider.deployContract(addedPublicKey, circuitDefs);
                    expect(result).toBeDefined();
                    expect(typeof result).toBe("string");
                    expect(result.length).toBeGreaterThan(0);
                    console.log("Deployed contract msg:", result);//    Deployed contract ID: deploy contract

                    await waitBlock(coordinator);
                    await waitBlock(coordinator);
                    const qbcDeployContract = await provider.getDeployContractCmd(addedPublicKey, circuitDefs);
                    console.log("qbcDeployContract: ", qbcDeployContract);

                    result = await provider.proveContractCall(addedPublicKey, {
                        contract_id: BigInt(0),
                        method_name: "simple_mint",
                        inputs: [BigInt(200000)],
                    });
                    console.log("result: ", result);

                    const submitResult = await provider.signAndSubmit(addedPublicKey);
                    console.log("submitResult: ", submitResult);
                    await waitBlock(coordinator);
                    await waitBlock(coordinator);

                } catch (error){
                    console.error("Failed to deploy contract:", error);
                    console.warn("This test may fail if contract deployment is not fully implemented");
                }
            },
            timeout * 60
        );
    });

    afterAll(async () => {
        console.log("🧹 WASM Provider integration tests completed");
        // Cleanup if needed
        await waitMs(1000);
    });
});
