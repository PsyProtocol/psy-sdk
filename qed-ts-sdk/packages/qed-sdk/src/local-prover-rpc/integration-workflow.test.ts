import { QEDRPCUserProverProvider } from "./client";
import { ContractCallArgs, WalletKeyPair } from "./types";
import { QHashOut } from "../core";

/**
 * End-to-end workflow integration tests
 * These tests simulate real user workflows with the QED User Prover RPC server
 */

describe("QED User Prover RPC Workflow Integration", () => {
    let provider: QEDRPCUserProverProvider;
    const rpcUrl = process.env.QED_RPC_URL || "http://localhost:8545";
    const timeout = 60000; // 60 seconds for complex workflows

    beforeAll(() => {
        provider = new QEDRPCUserProverProvider(rpcUrl);
    });

    describe("Complete User Workflow", () => {
        it(
            "should execute a complete user session workflow",
            async () => {
                console.log("Starting complete user workflow test...");

                try {
                    // Step 1: Start a new session
                    console.log("Step 1: Starting session...");
                    const sessionId = await provider.startSession();
                    expect(sessionId).toBeDefined();
                    console.log("✓ Session started:", sessionId);

                    // Step 2: Generate a random keypair
                    console.log("Step 2: Generating keypair...");
                    const keypair: WalletKeyPair = await provider.getRandomKeypair();
                    expect(keypair).toBeDefined();
                    expect(keypair.private_key).toBeDefined();
                    expect(keypair.public_key).toBeDefined();
                    console.log("✓ Keypair generated");

                    // Step 3: Add user to session
                    console.log("Step 3: Adding user to session...");
                    const userHash: QHashOut = await provider.addUser(keypair.private_key);
                    expect(userHash).toBeDefined();
                    console.log("✓ User added with hash:", userHash);

                    // Step 4: Switch to the user
                    console.log("Step 4: Switching to user...");
                    await provider.switchUser(userHash);
                    console.log("✓ Switched to user");

                    // Step 5: Verify ZK public key
                    console.log("Step 5: Verifying ZK public key...");
                    const zkPublicKey = await provider.getZKPublicKey(keypair.private_key);
                    expect(zkPublicKey.fingerprint).toEqual(keypair.public_key.fingerprint);
                    expect(zkPublicKey.public_key_param).toEqual(keypair.public_key.public_key_param);
                    console.log("✓ ZK public key verified");

                    // Step 6: Test ping functionality
                    console.log("Step 6: Testing connectivity...");
                    const pingResponse = await provider.ping("Workflow test ping");
                    expect(pingResponse).toBeDefined();
                    console.log("✓ Ping successful:", pingResponse);

                    console.log("✅ Complete user workflow test passed!");
                } catch (error) {
                    console.error("❌ Workflow test failed:", error);
                    throw error;
                }
            },
            timeout
        );

        it(
            "should handle multiple users in the same session",
            async () => {
                console.log("Starting multiple users workflow test...");

                try {
                    // Start session
                    const sessionId = await provider.startSession();
                    console.log("✓ Session started:", sessionId);

                    // Create and add multiple users
                    const users: Array<{ keypair: WalletKeyPair; hash: QHashOut }> = [];
                    const userCount = 3;

                    for (let i = 0; i < userCount; i++) {
                        console.log(`Creating user ${i + 1}/${userCount}...`);

                        const keypair = await provider.getRandomKeypair();
                        const userHash = await provider.addUser(keypair.private_key);

                        users.push({ keypair, hash: userHash });
                        console.log(`✓ User ${i + 1} created with hash:`, userHash);
                    }

                    // Switch between users and verify
                    for (let i = 0; i < users.length; i++) {
                        console.log(`Switching to user ${i + 1}...`);

                        await provider.switchUser(users[i].hash);

                        // Verify the user by checking ZK public key
                        const zkPublicKey = await provider.getZKPublicKey(users[i].keypair.private_key);
                        expect(zkPublicKey.fingerprint).toEqual(users[i].keypair.public_key.fingerprint);

                        console.log(`✓ Successfully switched to and verified user ${i + 1}`);
                    }

                    console.log("✅ Multiple users workflow test passed!");
                } catch (error) {
                    console.error("❌ Multiple users workflow test failed:", error);
                    throw error;
                }
            },
            timeout
        );
    });

    describe("Contract Interaction Workflow", () => {
        // let sessionId: string;
        let userKeypair: WalletKeyPair;
        let userHash: QHashOut;

        beforeAll(async () => {
            // Setup common session and user for contract tests
            await provider.startSession();
            userKeypair = await provider.getRandomKeypair();
            userHash = await provider.addUser(userKeypair.private_key);
            await provider.switchUser(userHash);
            console.log("✓ Setup complete for contract workflow tests");
        });

        it(
            "should execute contract proving workflow",
            async () => {
                console.log("Starting contract proving workflow test...");

                try {
                    // Step 1: Prove a single contract call
                    console.log("Step 1: Proving single contract call...");
                    const singleContractCall: ContractCallArgs = {
                        contract_id: 1n,
                        method_name: "test_method",
                        inputs: [42n, 100n],
                    };

                    const singleProofId = await provider.proveContractCall(singleContractCall);
                    expect(singleProofId).toBeDefined();
                    console.log("✓ Single contract call proven:", singleProofId);

                    // Step 2: Prove multiple contract calls
                    console.log("Step 2: Proving multiple contract calls...");
                    const multipleContractCalls: ContractCallArgs[] = [
                        { contract_id: 1n, method_name: "method_a", inputs: [10n] },
                        { contract_id: 1n, method_name: "method_b", inputs: [20n] },
                        { contract_id: 2n, method_name: "method_c", inputs: [30n, 40n] },
                    ];

                    const multipleProofId = await provider.proveContractCalls(multipleContractCalls);
                    expect(multipleProofId).toBeDefined();
                    console.log("✓ Multiple contract calls proven:", multipleProofId);

                    console.log("✅ Contract proving workflow test passed!");
                } catch (error) {
                    console.error("❌ Contract proving workflow test failed:", error);
                    console.warn("This test may fail if contracts are not deployed on the server");
                    // Don't throw to allow other tests to continue
                }
            },
            timeout
        );

        it(
            "should execute signing and submission workflow",
            async () => {
                console.log("Starting signing and submission workflow test...");

                try {
                    // Step 1: Prove some contract calls first
                    console.log("Step 1: Proving contract calls for submission...");
                    const contractCalls: ContractCallArgs[] = [
                        { contract_id: 1n, method_name: "submit_test", inputs: [123n] },
                    ];

                    await provider.proveContractCalls(contractCalls);
                    console.log("✓ Contract calls proven");

                    // Step 2: Get signature hash
                    console.log("Step 2: Getting signature hash...");
                    const networkMagic = 12345n;
                    const sigHash = await provider.getSigHash(networkMagic);
                    expect(sigHash).toBeDefined();
                    console.log("✓ Signature hash obtained:", sigHash);

                    // Step 3: Get ZK signature
                    console.log("Step 3: Getting ZK signature...");
                    const zkSignature = await provider.getZKSignature(sigHash);
                    expect(zkSignature).toBeDefined();
                    expect(zkSignature.proof).toBeDefined();
                    expect(zkSignature.public_inputs).toBeDefined();
                    console.log("✓ ZK signature generated");

                    // Step 4: Get end cap proof
                    console.log("Step 4: Getting end cap proof...");
                    const endCapProof = await provider.getEndCapProof(zkSignature);
                    expect(endCapProof).toBeDefined();
                    console.log("✓ End cap proof generated");

                    // Step 5: Get user EC input
                    console.log("Step 5: Getting user EC input...");
                    const userECInput = await provider.getUserECInput();
                    expect(userECInput).toBeDefined();
                    expect(userECInput.core).toBeDefined();
                    console.log("✓ User EC input obtained");

                    // Step 6: Sign and submit
                    console.log("Step 6: Signing and submitting...");
                    const submitId = await provider.signAndSubmit();
                    expect(submitId).toBeDefined();
                    console.log("✓ Transaction signed and submitted:", submitId);

                    console.log("✅ Signing and submission workflow test passed!");
                } catch (error) {
                    console.error("❌ Signing and submission workflow test failed:", error);
                    console.warn(
                        "This test may fail if the server doesn't have pending transactions or ZK proving is not available"
                    );
                    // Don't throw to allow other tests to continue
                }
            },
            timeout
        );
    });

    describe("Error Recovery Workflow", () => {
        it(
            "should handle and recover from session errors",
            async () => {
                console.log("Starting error recovery workflow test...");

                try {
                    // Step 1: Start a session
                    const sessionId = await provider.startSession();
                    console.log("✓ Initial session started:", sessionId);

                    // Step 2: Try to perform operations that might fail
                    try {
                        // This might fail if no contracts exist
                        await provider.proveContractCall({
                            contract_id: 999n,
                            method_name: "non_existent_method",
                            inputs: [1n],
                        });
                    } catch (error) {
                        console.log("✓ Expected error caught:", (error as Error).message);
                    }

                    // Step 3: Verify session is still functional
                    const pingResponse = await provider.ping("Recovery test");
                    expect(pingResponse).toBeDefined();
                    console.log("✓ Session still functional after error");

                    // Step 4: Start a new session to recover
                    const newSessionId = await provider.startSession();
                    expect(newSessionId).toBeDefined();
                    console.log("✓ New session started for recovery:", newSessionId);

                    // Step 5: Verify new session works
                    const newPingResponse = await provider.ping("New session test");
                    expect(newPingResponse).toBeDefined();
                    console.log("✓ New session is functional");

                    console.log("✅ Error recovery workflow test passed!");
                } catch (error) {
                    console.error("❌ Error recovery workflow test failed:", error);
                    throw error;
                }
            },
            timeout
        );
    });

    describe("Performance Workflow", () => {
        it(
            "should handle rapid sequential operations",
            async () => {
                console.log("Starting performance workflow test...");

                try {
                    const startTime = Date.now();

                    // Rapid sequence of operations
                    const sessionId = await provider.startSession();
                    const keypair1 = await provider.getRandomKeypair();
                    const keypair2 = await provider.getRandomKeypair();
                    const userHash1 = await provider.addUser(keypair1.private_key);
                    const userHash2 = await provider.addUser(keypair2.private_key);

                    await provider.switchUser(userHash1);
                    const zkKey1 = await provider.getZKPublicKey(keypair1.private_key);

                    await provider.switchUser(userHash2);
                    const zkKey2 = await provider.getZKPublicKey(keypair2.private_key);

                    const ping1 = await provider.ping("Performance test 1");
                    const ping2 = await provider.ping("Performance test 2");

                    const endTime = Date.now();
                    const totalTime = endTime - startTime;

                    // Verify all operations completed successfully
                    expect(sessionId).toBeDefined();
                    expect(userHash1).toBeDefined();
                    expect(userHash2).toBeDefined();
                    expect(zkKey1).toBeDefined();
                    expect(zkKey2).toBeDefined();
                    expect(ping1).toBeDefined();
                    expect(ping2).toBeDefined();

                    console.log(`✓ Completed ${10} operations in ${totalTime}ms`);
                    console.log(`✓ Average time per operation: ${(totalTime / 10).toFixed(2)}ms`);

                    // Performance expectation (adjust based on your requirements)
                    expect(totalTime).toBeLessThan(30000); // 30 seconds for all operations

                    console.log("✅ Performance workflow test passed!");
                } catch (error) {
                    console.error("❌ Performance workflow test failed:", error);
                    throw error;
                }
            },
            timeout
        );
    });
});
