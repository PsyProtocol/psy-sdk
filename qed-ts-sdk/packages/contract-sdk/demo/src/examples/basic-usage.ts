// src/examples/basic-usage.ts
import { Contract, Signer } from '../../../generated';
import { createProvider } from '../providers';
import { config } from '../config';

async function basicUsageExample() {
    console.log('🚀 Basic SDK Usage Example');
    console.log('=========================\n');

    // Step 1: Create provider
    console.log('1️⃣ Creating RPC Provider...');
    const provider = createProvider('local');
    console.log(`   Connected to: ${config.rpc.url}\n`);

    // Step 2: Create signer (required for state-changing operations)
    console.log('2️⃣ Creating Signer...');
    // Use the currentKeyPair from config which handles the derivation
    const currentKeyPair = config.user.currentKeyPair;
    const signer = Signer.fromPublicKey(currentKeyPair.publicKey, provider);
    console.log(`   Public Key: ${currentKeyPair.publicKey}`);
    console.log(`   User ID: ${currentKeyPair.userId}`);
    console.log(`   Realm ID: ${currentKeyPair.realmId}\n`);

    // Step 3: Create contract instance with signer
    console.log('3️⃣ Creating Contract Instance...');
    const contract = new Contract(
        config.contract.userId,
        config.contract.id,
        signer  // Pass signer instead of provider for full functionality
    );
    console.log(`   User ID: ${config.contract.userId}`);
    console.log(`   Contract ID: ${config.contract.id}\n`);

    // Step 4: Read simple state variable (no signer needed for reads)
    console.log('4️⃣ Reading Balance...');
    try {
        const balance = await contract.balance;
        console.log(`   ✅ Balance: ${balance} tokens\n`);
    } catch (error) {
        console.error(`   ❌ Error reading balance:`, error instanceof Error ? error.message : String(error));
    }

    // Step 5: Access nested array data (no signer needed for reads)
    console.log('5️⃣ Accessing Array Data...');
    try {
        // Access user data at index 536870912
        // The array calculation works as follows:
        // 1. other_user_info base offset: 1
        // 2. Array index 536870912 with nth_size 2: 536870912 * 2 = 1073741824
        // 3. Total array element offset: 1 + 1073741824 = 1073741825
        // 4. amount_sent is at position 0 within the struct
        // 5. amount_claimed is at position 1 within the struct
        const userIndex = 536870912;
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
    console.log('6️⃣ Executing Contract Function...');
    try {
        await contract.simple_mint(1000n);
        console.log(`   ✅ Successfully minted 1000 tokens\n`);
    } catch (error) {
        console.error(`   ❌ Error executing function:`, error instanceof Error ? error.message : String(error));
    }

    // Step 7: Demonstrate read-only contract (optional)
    console.log('7️⃣ Read-Only Contract Example...');
    console.log('   Creating read-only contract (no signer)...');
    const readOnlyContract = new Contract(
        config.contract.userId,
        config.contract.id,
        provider  // Pass provider directly for read-only access
    );

    try {
        // Reading still works
        const balance = await readOnlyContract.balance;
        console.log(`   ✅ Can read balance: ${balance}`);

        // But state-changing functions will fail
        try {
            await readOnlyContract.simple_mint(100n);
        } catch (error) {
            console.log(`   ✅ Expected error for write operation without signer:`,
                error instanceof Error ? error.message : String(error));
        }
    } catch (error) {
        console.error(`   ❌ Unexpected error:`, error instanceof Error ? error.message : String(error));
    }

    console.log('\n✨ Basic example complete!\n');
}

// Run the example
if (require.main === module) {
    basicUsageExample()
        .then(() => process.exit(0))
        .catch((error) => {
            console.error('Fatal error:', error);
            process.exit(1);
        });
}

export { basicUsageExample };