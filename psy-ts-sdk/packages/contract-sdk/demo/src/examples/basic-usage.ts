// src/examples/basic-usage.ts
import { PsyTokenContract as Contract, Signer, PsyFixedArray, Felt } from "../../../generated";
import { createMemoryWalletProvider } from "../providers";
import { PsyNetworkConfig, networkConfig } from "../config";
import { SignType } from "@psy/psy-sdk";

const privateKey = "c71603f33a1144ca7953db0ab48808f4c4055e3364a246c33c18a9786cb0b359";
const signType = "zk" as SignType;
const zkFingerprint = "65e0169bfffd55f1c0ea9f76c111a5b15e652322ee253c1a9604a10d59066b50";
const contractId = 0;

async function basicUsageExample() {
    console.log("🚀 Basic SDK Usage Example");
    console.log("=========================\n");

    // Step 1: Create wallet provider
    console.log("1️⃣ Creating Memory Wallet Provider...");
    const provider = await createMemoryWalletProvider(networkConfig);
    console.log(`   Connected to coordinator:`, networkConfig.coordinator_configs);
    console.log(`   Connected to realm:`, networkConfig.realm_configs);

    const checkpointId = (await provider.coordinatorEdgeRpcProvider.getLatestBlockState()).checkpoint_id;

    // Step 2: add user
    console.log("2️⃣ Adding User...");
    console.log(`   Provider:`, provider.signerProvider);
    const publicKey = await provider.signerProvider.registerUser(privateKey, signType);
    await provider.signerProvider.importPrivateKey?.(privateKey, signType, zkFingerprint);
    console.log(`   Public Key: ${publicKey}`);

    const userId = await provider.coordinatorEdgeRpcProvider.getUserId(publicKey);
    console.log(`   User ID: ${userId}`);

    const userLeafData = await provider.realmEdgeRpcProvider.getRpcProviderByUserId(userId).getUserLeafData(checkpointId, userId);
    console.log(`   User data: ${userLeafData}\n`);


    // Step 3: Create contract instance with signer
    console.log("3️⃣ Creating Contract Instance...");
    const singer = {
        publicKey,
        provider,
    }
    const contract = new Contract(
        checkpointId,
        userId,
        contractId,
        singer // Pass signer instead of provider for full functionality
    );
    console.log(`   User ID: ${userId}`);
    console.log(`   Contract ID: ${contractId}\n`);

    // Step 4: Read simple state variable (no signer needed for reads)
    console.log("4️⃣ Reading Balance...");
    try {
        const balance = await contract.balance;
        console.log(`   ✅ Balance: ${balance} tokens\n`);
    } catch (error) {
        console.error(`   ❌ Error reading balance:`, error instanceof Error ? error.message : String(error));
    }

    // Step 5: Access nested array data (no signer needed for reads)
    console.log("5️⃣ Accessing Array Data...");
    try {
        // Access user data at index 1048576
        // The array calculation works as follows:
        // 1. other_user_info base offset: 1
        // 2. Array index 1048576 with nth_size 2: 1048576 * 2 = 16777216
        // 3. Total array element offset: 1 + 16777216 = 1073741825
        // 4. amount_sent is at position 0 within the struct
        // 5. amount_claimed is at position 1 within the struct
        const userIndex = 1048576;
        const userInfo = contract.other_user_info[userIndex];

        // These will read from offsets:
        // amount_sent: 1073741825 (slot 268435456, position 1)
        // amount_claimed: 1073741826 (slot 268435456, position 2)
        const amountSent = await userInfo.amount_sent;
        const amountClaimed = await userInfo.amount_claimed;

        console.log(`   User ${userIndex} Data:`);
        console.log(`   - Amount Sent: ${amountSent}`);
        console.log(`   - Amount Claimed: ${amountClaimed}`);
        console.log(`   - Unclaimed: ${amountSent - amountClaimed}\n`);
    } catch (error) {
        console.error(`   ❌ Error accessing array:`, error instanceof Error ? error.message : String(error));
    }

    // Step 6: Execute a function (signer required)
    console.log("6️⃣ Executing Contract Function...");
    try {
        const mintBeforeAmount = await contract.balance;
        console.log(`   ✅ Minting ${mintBeforeAmount} tokens before minting...`);
        const mintAmounts = 10000000000000n;

        await contract.simple_mint(mintAmounts);
        console.log(`   ✅ Successfully mint 1000000000 tokens to each recipient\n`);

        // Wait for balance to increase
        let attempts = 0;
        const checkAmounts = 9000000000000n;
        let mintAfterAmount = await contract.balance;
        while (Number(mintAfterAmount) < checkAmounts && attempts < 10) {
            await new Promise(resolve => setTimeout(resolve, 1000));
            mintAfterAmount = await contract.balance;
            attempts++;
        }
        if (Number(mintAfterAmount) <= checkAmounts) {
            throw new Error(`Balance ${mintAfterAmount} is still not greater than ${checkAmounts} after 10 attempts`);
        }
        console.log(`   ✅ Balance confirmed: ${mintAfterAmount}\n`);
    } catch (error) {
        console.error(`   ❌ Error executing function:`, error instanceof Error ? error.message : String(error));
    }

    // Step 7: Demonstrate read-only contract (optional)
    console.log("7️⃣ Read-Only Contract Example...");
    console.log("   Creating read-only contract (no signer)...");
    const readOnlyContract = new Contract(
        checkpointId,
        userId,
        contractId,
        provider // Pass provider directly for read-only access
    );

    try {
        // Reading still works
        const balance = await readOnlyContract.balance;
        console.log(`   ✅ Can read balance: ${balance}`);

        // But state-changing functions will fail
        try {
            const recipients = 1;
            const amounts = 1000000;

            await contract.simple_transfer(recipients, amounts);
        } catch (error) {
            console.log(
                `   ✅ Expected error for write operation without signer:`,
                error instanceof Error ? error.message : String(error)
            );
        }
    } catch (error) {
        console.error(`   ❌ Unexpected error:`, error instanceof Error ? error.message : String(error));
    }

    console.log("\n✨ Basic example complete!\n");
}

// Run the example
if (require.main === module) {
    basicUsageExample()
        .then(() => process.exit(0))
        .catch((error) => {
            console.error("Fatal error:", error);
            process.exit(1);
        });
}

export { basicUsageExample };
