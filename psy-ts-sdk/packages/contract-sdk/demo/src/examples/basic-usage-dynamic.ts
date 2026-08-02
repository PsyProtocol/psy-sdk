// src/examples/basic-usage-dynamic.ts
// Runtime-dynamic Contract usage (no codegen required)
import { Contract, Signer } from "../../../src";
import abi from "../../../abi/contract.abi.json";
import { createMemoryWalletProvider } from "../providers";
import { networkConfig } from "../config";
import { SignType } from "@psy-protocol/psy-sdk";

const privateKey = "c71603f33a1144ca7953db0ab48808f4c4055e3364a246c33c18a9786cb0b359";
const signType = "zk" as SignType;
const zkFingerprint = "65e0169bfffd55f1c0ea9f76c111a5b15e652322ee253c1a9604a10d59066b50";
const contractId = 0;

async function basicUsageDynamicExample() {
    console.log("🚀 Basic SDK Usage Example (Dynamic / Runtime)");
    console.log("===============================================\n");

    // Step 1: Create wallet provider
    console.log("1️⃣ Creating Memory Wallet Provider...");
    const provider = await createMemoryWalletProvider(networkConfig);
    console.log(`   Connected to coordinator:`, networkConfig.coordinator_configs);
    console.log(`   Connected to realm:`, networkConfig.realm_configs);

    // Step 2: add user
    console.log("2️⃣ Adding User...");
    const registrationStartCheckpoint = (await provider.coordinatorEdgeRpcProvider.getLatestBlockState()).checkpoint_id;
    const publicKey = await provider.signerProvider.registerUser(privateKey, signType);
    const { userId, checkpointId } = await waitForRegisteredUser(provider, publicKey, registrationStartCheckpoint);
    await provider.signerProvider.importPrivateKey?.(privateKey, signType, zkFingerprint);
    console.log(`   Found User ID: ${userId}`);
    console.log(`   Public Key: ${publicKey}`);

    const userLeafData = await provider.realmEdgeRpcProvider.getRpcProviderByUserId(userId).getUserLeafData(checkpointId, userId);
    console.log(`   User data: ${userLeafData}\n`);

    // Step 3: Debug RPC params before creating contract
    console.log("3️⃣ Debugging RPC params...");
    try {
        const contractLeafData = await provider.coordinatorEdgeRpcProvider.getContractLeafData(contractId);
        console.log(`   📊 ContractLeafData for contractId=${contractId}:`, contractLeafData);
        console.log(`   📊 state_tree_height:`, contractLeafData.state_tree_height, `(type: ${typeof contractLeafData.state_tree_height})`);
    } catch (e: any) {
        console.error(`   ❌ getContractLeafData failed:`, e?.message || e);
    }

    // Direct RPC call test: getUserContractStateTreeLeafHash
    try {
        const realmRpc = provider.realmEdgeRpcProvider.getRpcProviderByUserId(userId);
        console.log(`   🔍 Direct call: getUserContractStateTreeLeafHash(${checkpointId}, ${userId}, ${contractId}, 20, 0)`);
        const leafHash = await realmRpc.getUserContractStateTreeLeafHash(checkpointId, userId, contractId, 20, 0);
        console.log(`   ✅ Direct leafHash result:`, leafHash);
    } catch (e: any) {
        console.error(`   ❌ Direct getUserContractStateTreeLeafHash failed:`, e?.message || e);
    }

    // Direct RPC call test: getSlotValues with [0n]
    try {
        const realmRpc = provider.realmEdgeRpcProvider.getRpcProviderByUserId(userId);
        console.log(`   🔍 Direct call: getSlotValues(${checkpointId}, ${userId}, ${contractId}, 20, [0n])`);
        const slotValues = await realmRpc.getSlotValues(checkpointId, userId, contractId, 20, [0n]);
        console.log(`   ✅ Direct slotValues result:`, slotValues);
    } catch (e: any) {
        console.error(`   ❌ Direct getSlotValues failed:`, e?.message || e);
    }

    // Direct RPC call test: provider.getContractState
    try {
        console.log(`   🔍 Direct call: provider.getContractState(${checkpointId}, ${contractId}, ${userId}, [0n])`);
        const state = await provider.getContractState(checkpointId, contractId, userId, [0n]);
        console.log(`   ✅ Direct getContractState result:`, state);
    } catch (e: any) {
        console.error(`   ❌ Direct getContractState failed:`, e?.message || e);
    }
    console.log("");

    // Step 4: Create dynamic contract instance with signer
    console.log("4️⃣ Creating Dynamic Contract Instance...");
    const signer: Signer = new Signer(publicKey, provider);
    const contract = new Contract(contractId, abi, signer, { checkpointId, userId });
    console.log(`   User ID: ${userId}`);
    console.log(`   Contract ID: ${contractId}\n`);

    // Step 5: Read simple state variable
    console.log("5️⃣ Reading Balance...");
    try {
        const balance = await contract.balance;
        console.log(`   ✅ Balance: ${balance} tokens\n`);
    } catch (error) {
        console.error(`   ❌ Error reading balance:`, error instanceof Error ? error.message : String(error));
    }

    // Step 6: Access nested array data
    console.log("6️⃣ Accessing Array Data...");
    try {
        const userIndex = 1048576;
        const userInfo = contract.other_user_info[userIndex];
        const amountSent = await userInfo.amount_sent;
        const amountClaimed = await userInfo.amount_claimed;

        console.log(`   User ${userIndex} Data:`);
        console.log(`   - Amount Sent: ${amountSent}`);
        console.log(`   - Amount Claimed: ${amountClaimed}`);
        console.log(`   - Unclaimed: ${Number(amountSent) - Number(amountClaimed)}\n`);
    } catch (error) {
        console.error(`   ❌ Error accessing array:`, error instanceof Error ? error.message : String(error));
    }

    // Step 7: Execute a function (signer required)
    console.log("7️⃣ Executing Contract Function...");
    try {
        const mintBeforeAmount = await contract.balance;
        console.log(`   ✅ Balance before minting: ${mintBeforeAmount}`);
        const mintAmounts = 10000000000000n;

        await contract.simple_mint(mintAmounts);
        console.log(`   ✅ Successfully minted ${mintAmounts} tokens\n`);

        let attempts = 0;
        const currentBalance = BigInt(await contract.balance);
        const checkAmounts = currentBalance + mintAmounts - 5000000000000n - 1000n * 100n;
        let mintAfterAmount = BigInt(await contract.balance);

        while (mintAfterAmount < checkAmounts && attempts < 100) {
            try {
                await contract.updateToLatest();
                console.log(`   📍 Updated to checkpoint: ${contract.checkpointId}`);
            } catch (updateError) {
                const latestCheckpoint = (await provider.coordinatorEdgeRpcProvider.getLatestBlockState()).checkpoint_id;
                contract.updateCheckpoint(latestCheckpoint);
                console.log(`   📍 Fallback: Manually updated to checkpoint: ${contract.checkpointId}`);
            }

            await new Promise((resolve) => setTimeout(resolve, 1000));
            mintAfterAmount = BigInt(await contract.balance);
            console.log(`   ✅ Balance: ${mintAfterAmount}`);
            attempts++;
        }

        if (mintAfterAmount <= checkAmounts) {
            throw new Error(`Balance ${mintAfterAmount} is still not greater than ${checkAmounts} after 100 attempts`);
        }
        console.log(`   ✅ Balance confirmed: ${mintAfterAmount}\n`);
    } catch (error) {
        console.error(`   ❌ Error executing function:`, error instanceof Error ? error.message : String(error));
    }

    // Step 8: Read-only contract (no signer)
    console.log("8️⃣ Read-Only Contract Example...");
    console.log("   Creating read-only contract (no signer)...");
    const latestCheckpointId = (await provider.coordinatorEdgeRpcProvider.getLatestBlockState()).checkpoint_id;
    const readOnlyContract = new Contract(contractId, abi, provider, { checkpointId: latestCheckpointId, userId });

    try {
        const balance = await readOnlyContract.balance;
        console.log(`   ✅ Can read balance: ${balance}`);

        try {
            const recipients = 1;
            const amounts = 1000000;
            await readOnlyContract.simple_transfer(recipients, amounts);
            console.log(`   ✅ Successfully transferred ${amounts} tokens to ${recipients}\n`);
            const balanceAfterTransfer = await readOnlyContract.balance;
            console.log(`   ✅ Balance after transfer: ${balanceAfterTransfer}\n`);
        } catch (error) {
            console.log(
                `   ✅ Expected error for write operation without signer:`,
                error instanceof Error ? error.message : String(error)
            );
        }
    } catch (error) {
        console.error(`   ❌ Unexpected error:`, error instanceof Error ? error.message : String(error));
    }

    // Step 9: Multi-instance demonstration
    console.log("\n9️⃣ Multi-Instance Demonstration...");
    try {
        const latestCheckpointId = (await provider.coordinatorEdgeRpcProvider.getLatestBlockState()).checkpoint_id;
        const contractA = new Contract(0, abi, signer, { checkpointId: latestCheckpointId, userId });
        const contractB = new Contract(1, abi, signer, { checkpointId: latestCheckpointId, userId });
        const balanceA = await contractA.balance;
        const balanceB = await contractB.balance;
        console.log(`   ✅ Contract 0 balance: ${balanceA}`);
        console.log(`   ✅ Contract 1 balance: ${balanceB}`);
        console.log(`   ✅ Both contracts share the same provider but have different IDs\n`);
    } catch (error) {
        console.error(`   ❌ Error in multi-instance demo:`, error instanceof Error ? error.message : String(error));
    }

    console.log("✨ Dynamic contract example complete!\n");
}

// Run the example
if (require.main === module) {
    basicUsageDynamicExample()
        .then(() => process.exit(0))
        .catch((error) => {
            console.error("Fatal error:", error);
            process.exit(1);
        });
}

export { basicUsageDynamicExample };

async function waitForRegisteredUser(
    provider: Awaited<ReturnType<typeof createMemoryWalletProvider>>,
    publicKey: string,
    registrationStartCheckpoint: number,
    attempts = 60,
) {
    for (let i = 0; i < attempts; i++) {
        const latestCheckpoint = (await provider.coordinatorEdgeRpcProvider.getLatestBlockState()).checkpoint_id;
        const progressed = Number(latestCheckpoint) - Number(registrationStartCheckpoint);
        if (progressed >= 2) {
            try {
                const userId = await provider.coordinatorEdgeRpcProvider.getUserId(publicKey);
                return { userId: Number(userId), checkpointId: Number(latestCheckpoint) };
            } catch (error) {
                console.log(
                    `   Registration not visible yet at checkpoint ${latestCheckpoint}:`,
                    error instanceof Error ? error.message : String(error),
                );
            }
        }
        console.log(`   Waiting for registered user... checkpoint ${latestCheckpoint} (+${progressed}) (${i + 1}/${attempts})`);
        await sleep(1000);
    }

    throw new Error(`Failed to resolve registered user after ${attempts} attempts`);
}

function sleep(ms: number): Promise<void> {
    console.log(`Sleeping for ${ms} milliseconds...`);
    return new Promise((resolve) => setTimeout(resolve, ms));
}
