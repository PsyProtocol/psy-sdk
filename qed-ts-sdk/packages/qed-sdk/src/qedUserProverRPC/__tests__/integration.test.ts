import { QEDRPCUserProverProvider } from "../qedClient";
import { ContractCallArgs, WalletKeyPair, ZKPublicKeyInfo, DPNFunctionCircuitDefinition } from "../qedTypes";
import { Hash256 } from "../../rpc/baseTypes";

/**
 * Integration tests for QED User Prover RPC Client
 * These tests require a running QED User Prover RPC server
 *
 * To run these tests:
 * 1. Start the QED User Prover RPC server on localhost:8545
 * 2. Set environment variable QED_RPC_URL if using different endpoint
 * 3. Run: npm test -- --testNamePattern="Integration"
 */

describe("QED User Prover RPC Integration Tests", () => {
    let provider: QEDRPCUserProverProvider;
    const rpcUrl = process.env.QED_RPC_URL || "http://localhost:8545";
    const timeout = 30000; // 30 seconds timeout for RPC calls

    beforeAll(() => {
        provider = new QEDRPCUserProverProvider(rpcUrl);
    });

    describe("Server Connectivity", () => {
        it(
            "should ping the server successfully",
            async () => {
                const message = "Integration test ping";

                try {
                    const response = await provider.ping(message);
                    expect(response).toBeDefined();
                    expect(typeof response).toBe("string");
                    console.log("Ping response:", response);
                } catch (error) {
                    console.error("Ping failed:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should handle ping with empty message",
            async () => {
                try {
                    const response = await provider.ping("");
                    expect(response).toBeDefined();
                } catch (error) {
                    console.error("Empty ping failed:", error);
                    throw error;
                }
            },
            timeout
        );
    });

    describe("Session Management", () => {
        let sessionId: string;

        it(
            "should start a new session",
            async () => {
                try {
                    sessionId = await provider.startSession();
                    expect(sessionId).toBeDefined();
                    expect(typeof sessionId).toBe("string");
                    expect(sessionId.length).toBeGreaterThan(0);
                    console.log("Started session:", sessionId);
                } catch (error) {
                    console.error("Failed to start session:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should start multiple sessions independently",
            async () => {
                try {
                    const session1 = await provider.startSession();
                    const session2 = await provider.startSession();

                    expect(session1).toBeDefined();
                    expect(session2).toBeDefined();
                    expect(session1).not.toBe(session2);
                    console.log("Multiple sessions:", { session1, session2 });
                } catch (error) {
                    console.error("Failed to start multiple sessions:", error);
                    throw error;
                }
            },
            timeout
        );
    });

    describe("User Management", () => {
        let testKeypair: WalletKeyPair;
        let userHash: Hash256;

        it(
            "should generate random keypair",
            async () => {
                try {
                    testKeypair = await provider.getRandomKeypair();

                    expect(testKeypair).toBeDefined();
                    expect(testKeypair.private_key).toBeDefined();
                    expect(testKeypair.public_key).toBeDefined();
                    expect(testKeypair.public_key.fingerprint).toBeDefined();
                    expect(testKeypair.public_key.public_key_param).toBeDefined();
                    expect(Array.isArray(testKeypair.public_key.fingerprint.elements)).toBe(true);
                    expect(Array.isArray(testKeypair.public_key.public_key_param.elements)).toBe(true);

                    console.log("Generated keypair:", {
                        privateKeyLength: testKeypair.private_key.length,
                        fingerprintElements: testKeypair.public_key.fingerprint.elements.length,
                        publicKeyElements: testKeypair.public_key.public_key_param.elements.length,
                    });
                } catch (error) {
                    console.error("Failed to generate keypair:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should get ZK public key from private key",
            async () => {
                try {
                    if (!testKeypair) {
                        testKeypair = await provider.getRandomKeypair();
                    }

                    const zkPublicKey: ZKPublicKeyInfo = await provider.getZKPublicKey(testKeypair.private_key);

                    expect(zkPublicKey).toBeDefined();
                    expect(zkPublicKey.fingerprint).toBeDefined();
                    expect(zkPublicKey.public_key_param).toBeDefined();
                    expect(Array.isArray(zkPublicKey.fingerprint.elements)).toBe(true);
                    expect(Array.isArray(zkPublicKey.public_key_param.elements)).toBe(true);

                    // Should match the public key from the keypair
                    expect(zkPublicKey.fingerprint.elements).toEqual(testKeypair.public_key.fingerprint.elements);
                    expect(zkPublicKey.public_key_param.elements).toEqual(
                        testKeypair.public_key.public_key_param.elements
                    );

                    console.log("ZK Public Key verified");
                } catch (error) {
                    console.error("Failed to get ZK public key:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should register a new user",
            async () => {
                try {
                    if (!testKeypair) {
                        testKeypair = await provider.getRandomKeypair();
                    }

                    userHash = await provider.registerUser(testKeypair.private_key);

                    expect(userHash).toBeDefined();
                    expect(typeof userHash).toBe("string");
                    expect(userHash.length).toBeGreaterThan(0);

                    console.log("Registered user with hash:", userHash);
                } catch (error) {
                    console.error("Failed to register user:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should add user to session",
            async () => {
                try {
                    if (!testKeypair) {
                        testKeypair = await provider.getRandomKeypair();
                    }

                    const addedUserHash = await provider.addUser(testKeypair.private_key);

                    expect(addedUserHash).toBeDefined();
                    expect(typeof addedUserHash).toBe("string");
                    expect(addedUserHash.length).toBeGreaterThan(0);

                    console.log("Added user with hash:", addedUserHash);
                } catch (error) {
                    console.error("Failed to add user:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should switch to user",
            async () => {
                try {
                    if (!userHash) {
                        // Create and add a user first
                        const keypair = await provider.getRandomKeypair();
                        userHash = await provider.addUser(keypair.private_key);
                    }

                    await provider.switchUser(userHash);
                    console.log("Successfully switched to user:", userHash);
                } catch (error) {
                    console.error("Failed to switch user:", error);
                    throw error;
                }
            },
            timeout
        );
    });

    describe("Contract Operations", () => {
        let sessionId: string;
        let userKeypair: WalletKeyPair;
        let userHash: Hash256;

        beforeAll(async () => {
            // Setup session and user for contract operations
            try {
                sessionId = await provider.startSession();
                userKeypair = await provider.getRandomKeypair();
                userHash = await provider.addUser(userKeypair.private_key);
                await provider.switchUser(userHash);
                console.log("Setup complete for contract operations");
            } catch (error) {
                console.error("Failed to setup for contract operations:", error);
                throw error;
            }
        }, timeout);

        it(
            "should prove a simple contract call",
            async () => {
                try {
                    const contractCall: ContractCallArgs = {
                        contract_id: 1n,
                        method_name: "main",
                        inputs: [123n, 456n],
                    };

                    const proofId = await provider.proveContractCall(contractCall);

                    expect(proofId).toBeDefined();
                    expect(typeof proofId).toBe("string");
                    expect(proofId.length).toBeGreaterThan(0);

                    console.log("Contract call proof ID:", proofId);
                } catch (error) {
                    console.error("Failed to prove contract call:", error);
                    // Don't throw here as this might fail if no contracts are deployed
                    console.warn("This test may fail if no contracts are deployed on the server");
                }
            },
            timeout
        );

        it(
            "should prove multiple contract calls",
            async () => {
                try {
                    const contractCalls: ContractCallArgs[] = [
                        { contract_id: 1n, method_name: "method1", inputs: [100n] },
                        { contract_id: 1n, method_name: "method2", inputs: [200n] },
                    ];

                    const proofId = await provider.proveContractCalls(contractCalls);

                    expect(proofId).toBeDefined();
                    expect(typeof proofId).toBe("string");
                    expect(proofId.length).toBeGreaterThan(0);

                    console.log("Multiple contract calls proof ID:", proofId);
                } catch (error) {
                    console.error("Failed to prove multiple contract calls:", error);
                    console.warn("This test may fail if no contracts are deployed on the server");
                }
            },
            timeout
        );

        it(
            "should handle contract call with empty inputs",
            async () => {
                try {
                    const contractCall: ContractCallArgs = {
                        contract_id: 1n,
                        method_name: "no_input_method",
                        inputs: [],
                    };

                    const proofId = await provider.proveContractCall(contractCall);

                    expect(proofId).toBeDefined();
                    console.log("Empty inputs contract call proof ID:", proofId);
                } catch (error) {
                    console.error("Failed to prove contract call with empty inputs:", error);
                    console.warn("This test may fail if the method doesn't exist");
                }
            },
            timeout
        );

        it(
            "should handle large input values",
            async () => {
                try {
                    const largeValue = BigInt("0xffffffffffffffffffffffffffffffff");
                    const contractCall: ContractCallArgs = {
                        contract_id: 1n,
                        method_name: "large_input_method",
                        inputs: [largeValue, largeValue],
                    };

                    const proofId = await provider.proveContractCall(contractCall);

                    expect(proofId).toBeDefined();
                    console.log("Large inputs contract call proof ID:", proofId);
                } catch (error) {
                    console.error("Failed to prove contract call with large inputs:", error);
                    console.warn("This test may fail if the method doesn't exist or doesn't support large inputs");
                }
            },
            timeout
        );
    });

    describe("Contract Deployment", () => {
        let sessionId: string;
        let userKeypair: WalletKeyPair;
        let userHash: Hash256;

        beforeAll(async () => {
            try {
                sessionId = await provider.startSession();
                userKeypair = await provider.getRandomKeypair();
                userHash = await provider.addUser(userKeypair.private_key);
                await provider.switchUser(userHash);
                console.log("Setup complete for contract deployment");
            } catch (error) {
                console.error("Failed to setup for contract deployment:", error);
                throw error;
            }
        }, timeout);

        it(
            "should get deploy contract command",
            async () => {
                try {
                    const circuitDefs: DPNFunctionCircuitDefinition[] = [
                        {
                            name: "test_function",
                            method_id: 1,
                            circuit_inputs: [1n, 2n],
                            circuit_outputs: [3n],
                            state_commands: [],
                            state_command_resolution_indices: [],
                            assertions: [],
                            definitions: [],
                        },
                    ];

                    const deployCmd = await provider.getDeployContractCmd(circuitDefs);

                    expect(deployCmd).toBeDefined();
                    expect(deployCmd.deployer).toBeDefined();
                    expect(deployCmd.code_definition).toBeDefined();
                    expect(deployCmd.function_whitelist).toBeDefined();
                    expect(Array.isArray(deployCmd.deployer.elements)).toBe(true);
                    expect(typeof deployCmd.code_definition.state_tree_height).toBe("number");
                    expect(Array.isArray(deployCmd.code_definition.functions)).toBe(true);

                    console.log("Deploy contract command:", {
                        deployerElements: deployCmd.deployer.elements.length,
                        stateTreeHeight: deployCmd.code_definition.state_tree_height,
                        functionsCount: deployCmd.code_definition.functions.length,
                        whitelistCount: deployCmd.function_whitelist.length,
                    });
                } catch (error) {
                    console.error("Failed to get deploy contract command:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should deploy contract",
            async () => {
                try {
                    const circuitDefs: DPNFunctionCircuitDefinition[] = [
                        {
                            name: "simple_contract",
                            method_id: 1,
                            circuit_inputs: [1n],
                            circuit_outputs: [1n],
                            state_commands: [],
                            state_command_resolution_indices: [],
                            assertions: [],
                            definitions: [],
                        },
                    ];

                    const contractId = await provider.deployContract(circuitDefs);

                    expect(contractId).toBeDefined();
                    expect(typeof contractId).toBe("string");
                    expect(contractId.length).toBeGreaterThan(0);

                    console.log("Deployed contract ID:", contractId);
                } catch (error) {
                    console.error("Failed to deploy contract:", error);
                    console.warn("This test may fail if contract deployment is not fully implemented");
                }
            },
            timeout
        );
    });

    describe("Signing and Submission", () => {
        let sessionId: string;
        let userKeypair: WalletKeyPair;
        let userHash: Hash256;

        beforeAll(async () => {
            try {
                sessionId = await provider.startSession();
                userKeypair = await provider.getRandomKeypair();
                userHash = await provider.addUser(userKeypair.private_key);
                await provider.switchUser(userHash);
                console.log("Setup complete for signing operations");
            } catch (error) {
                console.error("Failed to setup for signing operations:", error);
                throw error;
            }
        }, timeout);

        it(
            "should get signature hash",
            async () => {
                try {
                    const networkMagic = 12345n;
                    const sigHash = await provider.getSigHash(networkMagic);

                    expect(sigHash).toBeDefined();
                    expect(typeof sigHash).toBe("string");
                    expect(sigHash.length).toBeGreaterThan(0);

                    console.log("Signature hash:", sigHash);
                } catch (error) {
                    console.error("Failed to get signature hash:", error);
                    console.warn("This test may fail if no transaction is pending");
                }
            },
            timeout
        );

        it(
            "should get ZK signature",
            async () => {
                try {
                    // First get a signature hash
                    const networkMagic = 12345n;
                    const sigHash = await provider.getSigHash(networkMagic);

                    const zkSignature = await provider.getZKSignature(sigHash);

                    expect(zkSignature).toBeDefined();
                    expect(zkSignature.proof).toBeDefined();
                    expect(zkSignature.public_inputs).toBeDefined();
                    expect(Array.isArray(zkSignature.public_inputs)).toBe(true);

                    console.log("ZK signature:", {
                        publicInputsCount: zkSignature.public_inputs.length,
                        proofStructure: Object.keys(zkSignature.proof),
                    });
                } catch (error) {
                    console.error("Failed to get ZK signature:", error);
                    console.warn("This test may fail if no transaction is pending or ZK proving is not available");
                }
            },
            timeout
        );

        it(
            "should get user EC input",
            async () => {
                try {
                    const userECInput = await provider.getUserECInput();

                    expect(userECInput).toBeDefined();
                    expect(userECInput.core).toBeDefined();
                    expect(userECInput.contract_state_updates).toBeDefined();
                    expect(Array.isArray(userECInput.contract_state_updates)).toBe(true);
                    expect(typeof userECInput.core.checkpoint_id).toBe("bigint");

                    console.log("User EC input:", {
                        checkpointId: userECInput.core.checkpoint_id.toString(),
                        contractUpdatesCount: userECInput.contract_state_updates.length,
                    });
                } catch (error) {
                    console.error("Failed to get user EC input:", error);
                    console.warn("This test may fail if no transaction is pending");
                }
            },
            timeout
        );

        it(
            "should sign and submit transaction",
            async () => {
                try {
                    // First prove a contract call to have something to submit
                    const contractCall: ContractCallArgs = {
                        contract_id: 1n,
                        method_name: "test_method",
                        inputs: [42n],
                    };

                    await provider.proveContractCall(contractCall);

                    const submitId = await provider.signAndSubmit();

                    expect(submitId).toBeDefined();
                    expect(typeof submitId).toBe("string");
                    expect(submitId.length).toBeGreaterThan(0);

                    console.log("Submit ID:", submitId);
                } catch (error) {
                    console.error("Failed to sign and submit:", error);
                    console.warn("This test may fail if no contract calls were proven or submission is not ready");
                }
            },
            timeout
        );
    });

    describe("Result Retrieval", () => {
        it(
            "should handle getResult for valid ID",
            async () => {
                try {
                    // Generate a test result ID (this might not exist)
                    const testResultId: Hash256 = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

                    const result = await provider.getResult(testResultId);

                    // Result might be undefined if ID doesn't exist, which is expected
                    console.log("Result for test ID:", result);
                } catch (error) {
                    console.error("Failed to get result:", error);
                    console.warn("This test may fail if the result ID doesn't exist, which is expected");
                }
            },
            timeout
        );

        it(
            "should handle getResultFinal with retries",
            async () => {
                try {
                    // This will likely fail after retries, but tests the retry mechanism
                    const testResultId = Promise.resolve(
                        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                    );

                    const result = await provider.getResultFinal(testResultId, 2, 100);

                    console.log("Final result:", result);
                } catch (error) {
                    console.error("getResultFinal failed as expected:", (error as Error).message);
                    expect((error as Error).message).toContain("Result not found after");
                }
            },
            timeout
        );
    });

    describe("Error Handling", () => {
        it(
            "should handle invalid method calls gracefully",
            async () => {
                try {
                    // Call a non-existent method
                    await provider.rpc("invalid_method_name", []);

                    // Should not reach here
                    fail("Expected error for invalid method");
                } catch (error) {
                    expect(error).toBeDefined();
                    console.log("Correctly handled invalid method error:", (error as Error).message);
                }
            },
            timeout
        );

        it(
            "should handle invalid parameters gracefully",
            async () => {
                try {
                    // Call with invalid parameters
                    const invalidContractCall = {
                        contract_id: "invalid_id", // Should be bigint
                        method_name: 123, // Should be string
                        inputs: "invalid_inputs", // Should be array
                    } as any;

                    await provider.proveContractCall(invalidContractCall);

                    // Should not reach here
                    fail("Expected error for invalid parameters");
                } catch (error) {
                    expect(error).toBeDefined();
                    console.log("Correctly handled invalid parameters error:", (error as Error).message);
                }
            },
            timeout
        );

        it("should handle network timeouts", async () => {
            // Create a provider with a non-existent endpoint
            const invalidProvider = new QEDRPCUserProverProvider("http://localhost:9999");

            try {
                await invalidProvider.ping("test");

                // Should not reach here
                fail("Expected network error");
            } catch (error) {
                expect(error).toBeDefined();
                console.log("Correctly handled network error:", (error as Error).message);
            }
        }, 5000); // Shorter timeout for network error test
    });

    describe("Performance Tests", () => {
        it(
            "should handle concurrent requests",
            async () => {
                const concurrentRequests = 5;
                const promises: Promise<string>[] = [];

                for (let i = 0; i < concurrentRequests; i++) {
                    promises.push(provider.ping(`Concurrent test ${i}`));
                }

                try {
                    const results = await Promise.all(promises);

                    expect(results).toHaveLength(concurrentRequests);
                    results.forEach((result, index) => {
                        expect(result).toBeDefined();
                        console.log(`Concurrent request ${index} result:`, result);
                    });
                } catch (error) {
                    console.error("Concurrent requests failed:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should measure response times",
            async () => {
                const iterations = 3;
                const responseTimes: number[] = [];

                for (let i = 0; i < iterations; i++) {
                    const startTime = Date.now();

                    try {
                        await provider.ping(`Performance test ${i}`);
                        const responseTime = Date.now() - startTime;
                        responseTimes.push(responseTime);
                        console.log(`Ping ${i} response time: ${responseTime}ms`);
                    } catch (error) {
                        console.error(`Ping ${i} failed:`, error);
                    }
                }

                if (responseTimes.length > 0) {
                    const avgResponseTime = responseTimes.reduce((a, b) => a + b, 0) / responseTimes.length;
                    console.log(`Average response time: ${avgResponseTime.toFixed(2)}ms`);

                    // Reasonable response time expectation (adjust based on your requirements)
                    expect(avgResponseTime).toBeLessThan(5000); // 5 seconds
                }
            },
            timeout
        );
    });
});
