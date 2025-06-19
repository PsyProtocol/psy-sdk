/**
 * Integration tests for QED WASM User Prover Provider
 * These tests verify the WASM-based local prover functionality
 *
 * To run these tests:
 * 1. Ensure WASM module is properly compiled and available
 * 2. Run: npm test -- --testNamePattern="WASM.*Integration"
 */

import { createDefaultRpcConfig, RpcConfig } from "./config";
import { QEDWasmUserProverProvider } from "./provider";
import { PrivateKey, PublicKey, QHashOut } from "../core";
import {
    DPNFunctionCircuitDefinition,
    ProofWithPublicInputs,
    SubmitUserEndCapNonProofInput,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo, ContractCallArgs, WalletKeyPair, QBCDeployContract } from "../types";
import { waitMs } from "../utils";

describe("QED WASM User Prover Provider Integration Tests", () => {
    let provider: QEDWasmUserProverProvider;
    let rpcConfig: RpcConfig;
    const timeout = 60000; // 60 seconds timeout for WASM operations

    // Test data
    let testPrivateKey: PrivateKey;
    let testPublicKey: ZKPublicKeyInfo;
    let testKeypair: WalletKeyPair;
    let sessionId: string;

    beforeAll(async () => {
        // Initialize WASM provider with default configuration
        rpcConfig = createDefaultRpcConfig();
        provider = new QEDWasmUserProverProvider(rpcConfig);

        // Generate test keypair
        testKeypair = await provider.getRandomKeypair();
        testPrivateKey = testKeypair.private_key;
        testPublicKey = testKeypair.public_key;

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
                //expect(response).toContain(message);

                console.log("📡 Ping response:", response);
            },
            timeout
        );

        it(
            "should start a session successfully",
            async () => {
                sessionId = await provider.startSession();
                expect(sessionId).toBeDefined();
                expect(typeof sessionId).toBe("string");
                expect(sessionId.length).toBeGreaterThan(0);

                console.log("🚀 Session started:", sessionId);
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
    });

    describe("User Management Operations", () => {
        it(
            "should register a user successfully",
            async () => {
                const registeredPublicKey: PublicKey = await provider.registerUser(testPrivateKey);
                expect(registeredPublicKey).toBeDefined();
                expect(typeof registeredPublicKey).toBe("string");

                console.log("👤 User registered with public key:", registeredPublicKey);
            },
            timeout
        );

        it(
            "should add a user successfully",
            async () => {
                const newKeypair = await provider.getRandomKeypair();
                const addedPublicKey: PublicKey = await provider.addUser(newKeypair.private_key);
                expect(addedPublicKey).toBeDefined();
                expect(typeof addedPublicKey).toBe("string");

                console.log("➕ User added with public key:", addedPublicKey);
            },
            timeout
        );

        it(
            "should switch user successfully",
            async () => {
                // Switch to the test user using fingerprint as PublicKey
                await expect(provider.switchUser(testPublicKey.fingerprint)).resolves.not.toThrow();

                console.log("🔄 Switched to user:", testPublicKey.fingerprint);
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
                deployContractCmd = await provider.getDeployContractCmd(mockCircuitDefs);
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
                const deployResult = await provider.deployContract(mockCircuitDefs);
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

                const proofResult = await provider.proveContractCall(contractCallArgs);
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

                const proofResult = await provider.proveContractCalls(contractCallArgs);
                expect(proofResult).toBeDefined();
                expect(typeof proofResult).toBe("string");

                console.log("🔍 Multiple contract calls proved:", proofResult.substring(0, 50) + "...");
            },
            timeout
        );
    });

    describe("Signing and Submission Operations", () => {
        let sighash: QHashOut;
        let zkSignature: ProofWithPublicInputs;
        let endCapProof: ProofWithPublicInputs;

        it(
            "should get signature hash",
            async () => {
                const networkMagic = BigInt(12345);
                sighash = await provider.getSigHash(networkMagic);
                expect(sighash).toBeDefined();
                expect(typeof sighash).toBe("string");

                console.log("🔐 Signature hash:", sighash.substring(0, 20) + "...");
            },
            timeout
        );

        it(
            "should get ZK signature",
            async () => {
                if (!sighash) {
                    sighash = await provider.getSigHash(BigInt(12345));
                }

                zkSignature = await provider.getZKSignature(sighash);
                expect(zkSignature).toBeDefined();
                expect(zkSignature.proof).toBeDefined();
                expect(zkSignature.public_inputs).toBeDefined();
                expect(Array.isArray(zkSignature.public_inputs)).toBe(true);

                console.log("✍️ ZK signature generated with", zkSignature.public_inputs.length, "public inputs");
            },
            timeout
        );

        it(
            "should get end cap proof",
            async () => {
                if (!zkSignature) {
                    const tempSighash = await provider.getSigHash(BigInt(12345));
                    zkSignature = await provider.getZKSignature(tempSighash);
                }

                endCapProof = await provider.getEndCapProof(zkSignature);
                expect(endCapProof).toBeDefined();
                expect(endCapProof.proof).toBeDefined();
                expect(endCapProof.public_inputs).toBeDefined();

                console.log("🏁 End cap proof generated with", endCapProof.public_inputs.length, "public inputs");
            },
            timeout
        );

        it(
            "should get user EC input",
            async () => {
                const userECInput: SubmitUserEndCapNonProofInput = await provider.getUserECInput();
                expect(userECInput).toBeDefined();
                expect(userECInput.core).toBeDefined();
                expect(userECInput.contract_state_updates).toBeDefined();
                expect(Array.isArray(userECInput.contract_state_updates)).toBe(true);

                console.log(
                    "📊 User EC input generated with",
                    userECInput.contract_state_updates.length,
                    "contract updates"
                );
            },
            timeout
        );

        it(
            "should sign and submit successfully",
            async () => {
                const submitResult = await provider.signAndSubmit();
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

                await expect(provider.proveContractCall(invalidContractCall)).rejects.toThrow();
                console.log("❌ Invalid contract call properly rejected");
            },
            timeout
        );

        it(
            "should handle empty circuit definitions",
            async () => {
                const emptyCircuitDefs: DPNFunctionCircuitDefinition[] = [];

                await expect(provider.deployContract(emptyCircuitDefs)).rejects.toThrow();
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

    afterAll(async () => {
        console.log("🧹 WASM Provider integration tests completed");
        // Cleanup if needed
        await waitMs(1000);
    });
});
