/**
 * QED WASM Provider Usage Examples
 * 
 * This file demonstrates how to use the QED WASM Provider in different scenarios,
 * including direct usage and Web Worker integration.
 */

import {
  QEDWasmProvider,
  QEDWasmWorkerProvider,
  createQEDWasmProvider,
  type QEDWasmProviderConfig,
} from './qed-wasm-provider';

/**
 * Example 1: Basic Direct Usage
 * Use this approach for simple applications or when you don't need background processing.
 */
export async function basicUsageExample() {
  console.log('=== Basic Usage Example ===');
  
  // Create and initialize the provider
  const config: QEDWasmProviderConfig = {
    debug: true,
    useWorker: false, // Direct usage without worker
  };
  
  const provider = await createQEDWasmProvider(config);
  
  try {
    // Test basic connectivity
    const pingResult = await provider.ping();
    console.log('Ping result:', pingResult);
    
    // Start a session
    const sessionId = await provider.startSession();
    console.log('Session started:', sessionId);
    
    // Generate a random keypair
    const keypair = await provider.getRandomKeypair();
    console.log('Generated keypair:', {
      publicKey: keypair.publicKey.substring(0, 20) + '...',
      privateKey: '[HIDDEN]',
    });
    
    // Register a user
    const userId = 'user_' + Date.now();
    const registered = await provider.registerUser(userId, keypair.publicKey);
    console.log('User registered:', registered);
    
    // Add user with private key
    const added = await provider.addUser(userId, keypair.privateKey);
    console.log('User added:', added);
    
    // Switch to the user
    const switched = await provider.switchUser(userId);
    console.log('User switched:', switched);
    
    // Get ZK public key
    const zkPublicKey = await provider.getZKPublicKey();
    console.log('ZK Public Key:', zkPublicKey.substring(0, 20) + '...');
    
  } catch (error) {
    console.error('Error in basic usage example:', error);
  } finally {
    // Cleanup
    await provider.dispose();
  }
}

/**
 * Example 2: Web Worker Usage
 * Use this approach for intensive computations that shouldn't block the UI.
 */
export async function workerUsageExample() {
  console.log('=== Worker Usage Example ===');
  
  // Create worker provider
  const workerProvider = new QEDWasmWorkerProvider('./qed-wasm-worker.js');
  
  try {
    // Initialize the worker
    await workerProvider.initialize();
    console.log('Worker initialized');
    
    // Test basic connectivity
    const pingResult = await workerProvider.ping();
    console.log('Worker ping result:', pingResult);
    
    // Start a session
    const sessionId = await workerProvider.startSession();
    console.log('Worker session started:', sessionId);
    
    // Generate keypair in worker
    const keypair = await workerProvider.getRandomKeypair();
    console.log('Worker generated keypair:', {
      publicKey: keypair.publicKey.substring(0, 20) + '...',
      privateKey: '[HIDDEN]',
    });
    
    // Check worker status
    console.log('Worker initialized:', workerProvider.isInitialized());
    console.log('Pending operations:', workerProvider.getPendingOperationCount());
    
  } catch (error) {
    console.error('Error in worker usage example:', error);
  } finally {
    // Cleanup worker
    await workerProvider.dispose();
  }
}

/**
 * Example 3: Contract Proving
 * Demonstrates how to prove contract calls using the WASM provider.
 */
export async function contractProvingExample() {
  console.log('=== Contract Proving Example ===');
  
  const provider = await createQEDWasmProvider({ debug: true });
  
  try {
    // Start session and setup user
    const sessionId = await provider.startSession();
    const keypair = await provider.getRandomKeypair();
    const userId = 'prover_' + Date.now();
    
    await provider.registerUser(userId, keypair.publicKey);
    await provider.addUser(userId, keypair.privateKey);
    await provider.switchUser(userId);
    
    // Define a simple circuit
    const circuitDef = {
      function_name: 'add',
      input_vars: [
        { var_name: 'a', var_index: 0 },
        { var_name: 'b', var_index: 1 },
      ],
      output_vars: [
        { var_name: 'result', var_index: 2 },
      ],
      assert_eq_infos: [
        { lhs: 2, rhs: 0 }, // result = a + b (simplified)
      ],
      state_cmds: [],
    };
    
    // Prove a contract call
    console.log('Generating proof for contract call...');
    const proof = await provider.proveContractCall(
      '0x1234567890abcdef', // contract address
      'add', // function name
      ['10', '20'], // arguments
      circuitDef
    );
    
    console.log('Proof generated successfully!');
    console.log('Public inputs:', proof.public_inputs);
    
    // Sign and submit the proof
    const transactionData = JSON.stringify({
      to: '0x1234567890abcdef',
      data: 'add(10,20)',
      gas: 100000,
    });
    
    const submitResult = await provider.signAndSubmit(proof, transactionData);
    console.log('Transaction submitted:', submitResult);
    
  } catch (error) {
    console.error('Error in contract proving example:', error);
  } finally {
    await provider.dispose();
  }
}

/**
 * Example 4: Contract Deployment
 * Shows how to deploy contracts using the WASM provider.
 */
export async function contractDeploymentExample() {
  console.log('=== Contract Deployment Example ===');
  
  const provider = await createQEDWasmProvider({ debug: true });
  
  try {
    // Setup user
    const sessionId = await provider.startSession();
    const keypair = await provider.getRandomKeypair();
    const userId = 'deployer_' + Date.now();
    
    await provider.registerUser(userId, keypair.publicKey);
    await provider.addUser(userId, keypair.privateKey);
    await provider.switchUser(userId);
    
    // Define contract code
    const contractCode = {
      contract_name: 'SimpleCalculator',
      functions: [
        {
          function_name: 'add',
          circuit_def: {
            function_name: 'add',
            input_vars: [
              { var_name: 'a', var_index: 0 },
              { var_name: 'b', var_index: 1 },
            ],
            output_vars: [
              { var_name: 'result', var_index: 2 },
            ],
            assert_eq_infos: [
              { lhs: 2, rhs: 0 },
            ],
            state_cmds: [],
          },
        },
        {
          function_name: 'multiply',
          circuit_def: {
            function_name: 'multiply',
            input_vars: [
              { var_name: 'x', var_index: 0 },
              { var_name: 'y', var_index: 1 },
            ],
            output_vars: [
              { var_name: 'product', var_index: 2 },
            ],
            assert_eq_infos: [
              { lhs: 2, rhs: 0 },
            ],
            state_cmds: [],
          },
        },
      ],
    };
    
    // Get deployment command
    const deployCmd = await provider.getDeployContractCmd(
      contractCode,
      ['0'] // constructor args
    );
    console.log('Deploy command:', deployCmd);
    
    // Deploy the contract
    const deployResult = await provider.deployContract(
      contractCode,
      ['0']
    );
    console.log('Contract deployed:', deployResult);
    
  } catch (error) {
    console.error('Error in contract deployment example:', error);
  } finally {
    await provider.dispose();
  }
}

/**
 * Example 5: Batch Operations
 * Demonstrates proving multiple contract calls in a single batch.
 */
export async function batchOperationsExample() {
  console.log('=== Batch Operations Example ===');
  
  const provider = await createQEDWasmProvider({ debug: true });
  
  try {
    // Setup
    const sessionId = await provider.startSession();
    const keypair = await provider.getRandomKeypair();
    const userId = 'batch_user_' + Date.now();
    
    await provider.registerUser(userId, keypair.publicKey);
    await provider.addUser(userId, keypair.privateKey);
    await provider.switchUser(userId);
    
    // Define multiple calls
    const calls = [
      {
        contractAddress: '0x1111111111111111',
        functionName: 'add',
        args: ['5', '3'],
        circuitDef: {
          function_name: 'add',
          input_vars: [{ var_name: 'a', var_index: 0 }, { var_name: 'b', var_index: 1 }],
          output_vars: [{ var_name: 'result', var_index: 2 }],
          assert_eq_infos: [{ lhs: 2, rhs: 0 }],
          state_cmds: [],
        },
      },
      {
        contractAddress: '0x2222222222222222',
        functionName: 'multiply',
        args: ['4', '7'],
        circuitDef: {
          function_name: 'multiply',
          input_vars: [{ var_name: 'x', var_index: 0 }, { var_name: 'y', var_index: 1 }],
          output_vars: [{ var_name: 'product', var_index: 2 }],
          assert_eq_infos: [{ lhs: 2, rhs: 0 }],
          state_cmds: [],
        },
      },
    ];
    
    // Prove batch of calls
    console.log('Generating batch proof...');
    const batchProof = await provider.proveContractCalls(calls);
    console.log('Batch proof generated successfully!');
    console.log('Batch public inputs:', batchProof.public_inputs);
    
  } catch (error) {
    console.error('Error in batch operations example:', error);
  } finally {
    await provider.dispose();
  }
}

/**
 * Run all examples
 */
export async function runAllExamples() {
  console.log('Running QED WASM Provider Examples...');
  
  try {
    await basicUsageExample();
    console.log('\n');
    
    await workerUsageExample();
    console.log('\n');
    
    await contractProvingExample();
    console.log('\n');
    
    await contractDeploymentExample();
    console.log('\n');
    
    await batchOperationsExample();
    
    console.log('\nAll examples completed successfully!');
  } catch (error) {
    console.error('Error running examples:', error);
  }
}

// Export individual examples for selective testing
export {
  basicUsageExample,
  workerUsageExample,
  contractProvingExample,
  contractDeploymentExample,
  batchOperationsExample,
};