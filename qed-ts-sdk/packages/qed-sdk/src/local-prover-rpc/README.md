# QED User Prover RPC Client

This module provides TypeScript clients for interacting with the QED User Prover RPC service.

## Installation

```bash
npm install @qed/sdk
```

## Usage

### QED API

```typescript
import { QEDRPCUserProverProvider } from "@qed/sdk/userProverRPC";

// Create a new RPC client
const userProverClient = new QEDRPCUserProverProvider("http://localhost:8545");

// Start a new session
async function startSession() {
    try {
        const result = await userProverClient.startSession();
        console.log("Session started:", result);
    } catch (error) {
        console.error("Error starting session:", error);
    }
}

// Prove a contract call
async function proveContract() {
    try {
        const contractCallArg = {
            contract_id: 1n,
            method_name: "main",
            inputs: [123n, 456n],
        };

        const result = await userProverClient.proveContractCall(contractCallArg);
        console.log("Contract call proven:", result);

        // Sign and submit
        const submitResult = await userProverClient.signAndSubmit();
        console.log("Signed and submitted:", submitResult);
    } catch (error) {
        console.error("Error proving contract call:", error);
    }
}

// User operations
async function userOperations() {
    try {
        // Generate a random keypair
        const keypair = await userProverClient.getRandomKeypair();
        console.log("Random keypair:", keypair);

        // Add user
        const userHash = await userProverClient.addUser(keypair.private_key);
        console.log("User added, hash:", userHash);

        // Switch to this user
        await userProverClient.switchUser(userHash);
        console.log("Switched to user");
    } catch (error) {
        console.error("Error with user operations:", error);
    }
}

// Run all operations
async function runExample() {
    await startSession();
    await userOperations();
    await proveContract();
}

runExample();
```

## API Reference

### QEDRPCUserProverProvider

This class implements the Rust RPC server functionality in TypeScript.

Key methods:

- `startSession()`: Start a new proving session
- `proveContractCall(contractCallArg)`: Prove a contract call
- `proveContractCalls(contractCallArgs)`: Prove multiple contract calls
- `signAndSubmit()`: Sign the transaction and submit it
- `registerUser(privateKey)`: Register a new user
- `addUser(privateKey)`: Add a user to the session
- `switchUser(pkHash)`: Switch to a different user
- `getZKPublicKey(privateKey)`: Get a ZK public key
- `getRandomKeypair()`: Generate a random keypair
- `deployContract(circuitDefs)`: Deploy a contract
- `getDeployContractCmd(circuitDefs)`: Get the deploy contract command
- `getSigHash(networkMagic)`: Get the signature hash
- `getZKSignature(sighash)`: Get a ZK signature
- `getEndCapProof(signatureProof)`: Get the end cap proof
- `getUserECInput()`: Get the user EC input
- `ping(message)`: Test connectivity with the server
- `getResult(id)`: Get the result of a previous operation

For more details, see the TypeScript definitions and Rust server implementation.
