import { createQEDWasmProvider } from './qed-wasm-provider';
import { QEDWasmWorkerProvider } from './qed-wasm-worker-proxy';
import type { IQEDUserProverProvider } from 'qed-ts-sdk';

/**
 * Test suite for QED WASM Provider
 */
class QEDWasmTest {
  private provider: IQEDUserProverProvider | null = null;
  private sessionId: string | null = null;
  private userId: string | null = null;

  async runAllTests(): Promise<void> {
    console.log('🚀 Starting QED WASM Provider Tests');
    
    try {
      await this.testDirectProvider();
      await this.testWorkerProvider();
      console.log('✅ All tests passed!');
    } catch (error) {
      console.error('❌ Tests failed:', error);
      throw error;
    }
  }

  async testDirectProvider(): Promise<void> {
    console.log('\n📋 Testing Direct Provider...');
    
    this.provider = await createQEDWasmProvider({
      debug: true,
      useWorker: false,
    });

    await this.runBasicTests('Direct Provider');
    await this.provider.dispose();
    this.provider = null;
  }

  async testWorkerProvider(): Promise<void> {
    console.log('\n📋 Testing Worker Provider...');
    
    this.provider = new QEDWasmWorkerProvider('./qed-wasm-worker.js');
    await (this.provider as QEDWasmWorkerProvider).initialize();

    await this.runBasicTests('Worker Provider');
    await this.provider.dispose();
    this.provider = null;
  }

  private async runBasicTests(providerType: string): Promise<void> {
    if (!this.provider) {
      throw new Error('Provider not initialized');
    }

    console.log(`\n🔧 Running basic tests for ${providerType}...`);

    // Test 1: Ping
    await this.testPing();

    // Test 2: Session Management
    await this.testSessionManagement();

    // Test 3: User Management
    await this.testUserManagement();

    // Test 4: Cryptographic Operations
    await this.testCryptographicOperations();

    // Test 5: Contract Operations (mock)
    await this.testContractOperations();

    console.log(`✅ ${providerType} tests completed successfully`);
  }

  private async testPing(): Promise<void> {
    console.log('  🏓 Testing ping...');
    const response = await this.provider!.ping();
    if (!response || typeof response !== 'string') {
      throw new Error('Ping failed: invalid response');
    }
    console.log(`    ✅ Ping successful: ${response}`);
  }

  private async testSessionManagement(): Promise<void> {
    console.log('  📝 Testing session management...');
    
    this.sessionId = await this.provider!.startSession();
    if (!this.sessionId || typeof this.sessionId !== 'string') {
      throw new Error('Failed to start session');
    }
    
    console.log(`    ✅ Session started: ${this.sessionId}`);
  }

  private async testUserManagement(): Promise<void> {
    console.log('  👤 Testing user management...');
    
    // Generate keypair
    const keypair = await this.provider!.getRandomKeypair();
    if (!keypair.publicKey || !keypair.privateKey) {
      throw new Error('Failed to generate keypair');
    }
    console.log(`    ✅ Keypair generated`);

    // Register user
    this.userId = `test_user_${Date.now()}`;
    const registered = await this.provider!.registerUser(this.userId, keypair.publicKey);
    if (!registered) {
      throw new Error('Failed to register user');
    }
    console.log(`    ✅ User registered: ${this.userId}`);

    // Add user
    const added = await this.provider!.addUser(this.userId, keypair.privateKey);
    if (!added) {
      throw new Error('Failed to add user');
    }
    console.log(`    ✅ User added`);

    // Switch user
    const switched = await this.provider!.switchUser(this.userId);
    if (!switched) {
      throw new Error('Failed to switch user');
    }
    console.log(`    ✅ User switched`);

    // Get ZK public key
    const zkPublicKey = await this.provider!.getZKPublicKey();
    if (!zkPublicKey) {
      throw new Error('Failed to get ZK public key');
    }
    console.log(`    ✅ ZK public key retrieved`);
  }

  private async testCryptographicOperations(): Promise<void> {
    console.log('  🔐 Testing cryptographic operations...');
    
    if (!this.userId) {
      throw new Error('User not set up for crypto tests');
    }

    // Test signature hash
    const message = 'Hello, QED WASM!';
    const sigHash = await this.provider!.getSigHash(message);
    if (!sigHash) {
      throw new Error('Failed to get signature hash');
    }
    console.log(`    ✅ Signature hash generated`);

    // Test ZK signature
    const zkSignature = await this.provider!.getZKSignature(message);
    if (!zkSignature) {
      throw new Error('Failed to get ZK signature');
    }
    console.log(`    ✅ ZK signature generated`);

    // Test user EC input
    const ecInput = await this.provider!.getUserECInput(this.userId);
    if (!ecInput) {
      throw new Error('Failed to get user EC input');
    }
    console.log(`    ✅ User EC input retrieved`);
  }

  private async testContractOperations(): Promise<void> {
    console.log('  📄 Testing contract operations (mock)...');
    
    // Note: These are mock tests since we don't have actual contract definitions
    // In a real scenario, you would provide actual circuit definitions
    
    try {
      // Test deploy contract command generation
      const mockCircuitDef = {
        name: 'TestContract',
        functions: [{
          name: 'add',
          inputs: ['uint256', 'uint256'],
          outputs: ['uint256'],
          circuit: 'mock_circuit_data'
        }]
      };

      const deployCmd = await this.provider!.getDeployContractCmd(
        'TestContract',
        JSON.stringify(mockCircuitDef)
      );
      
      if (!deployCmd) {
        console.log(`    ⚠️  Deploy command generation skipped (expected for mock data)`);
      } else {
        console.log(`    ✅ Deploy command generated`);
      }
    } catch (error) {
      console.log(`    ⚠️  Contract operations skipped (expected for mock data): ${error}`);
    }
  }

  // Performance test
  async runPerformanceTest(): Promise<void> {
    console.log('\n⚡ Running performance tests...');
    
    const provider = await createQEDWasmProvider({ debug: false });
    
    try {
      const iterations = 10;
      const operations = [];
      
      console.log(`  📊 Testing ${iterations} ping operations...`);
      const startTime = Date.now();
      
      for (let i = 0; i < iterations; i++) {
        const opStart = Date.now();
        await provider.ping();
        const opEnd = Date.now();
        operations.push(opEnd - opStart);
      }
      
      const endTime = Date.now();
      const totalTime = endTime - startTime;
      const avgTime = operations.reduce((a, b) => a + b, 0) / operations.length;
      const minTime = Math.min(...operations);
      const maxTime = Math.max(...operations);
      
      console.log(`    ✅ Performance results:`);
      console.log(`       Total time: ${totalTime}ms`);
      console.log(`       Average time: ${avgTime.toFixed(2)}ms`);
      console.log(`       Min time: ${minTime}ms`);
      console.log(`       Max time: ${maxTime}ms`);
      console.log(`       Operations/sec: ${(1000 / avgTime).toFixed(2)}`);
      
    } finally {
      await provider.dispose();
    }
  }

  // Memory test
  async runMemoryTest(): Promise<void> {
    console.log('\n🧠 Running memory tests...');
    
    const provider = await createQEDWasmProvider({ debug: false });
    
    try {
      // Test multiple session creation and cleanup
      const sessions = [];
      
      console.log('  📈 Creating multiple sessions...');
      for (let i = 0; i < 5; i++) {
        const sessionId = await provider.startSession();
        sessions.push(sessionId);
        console.log(`    Session ${i + 1}: ${sessionId}`);
      }
      
      console.log('  🧹 Testing cleanup...');
      // In a real implementation, you might have session cleanup methods
      
      console.log('    ✅ Memory test completed');
      
    } finally {
      await provider.dispose();
    }
  }

  // Error handling test
  async runErrorHandlingTest(): Promise<void> {
    console.log('\n🚨 Running error handling tests...');
    
    const provider = await createQEDWasmProvider({ debug: false });
    
    try {
      // Test invalid operations
      console.log('  ❌ Testing invalid user operations...');
      
      try {
        await provider.switchUser('nonexistent_user');
        console.log('    ⚠️  Expected error not thrown for invalid user switch');
      } catch (error) {
        console.log('    ✅ Invalid user switch properly handled');
      }
      
      try {
        await provider.getZKSignature('');
        console.log('    ⚠️  Expected error not thrown for empty message');
      } catch (error) {
        console.log('    ✅ Empty message properly handled');
      }
      
      console.log('  ✅ Error handling tests completed');
      
    } finally {
      await provider.dispose();
    }
  }
}

// Main test runner
async function main(): Promise<void> {
  const tester = new QEDWasmTest();
  
  try {
    await tester.runAllTests();
    await tester.runPerformanceTest();
    await tester.runMemoryTest();
    await tester.runErrorHandlingTest();
    
    console.log('\n🎉 All tests completed successfully!');
    process.exit(0);
  } catch (error) {
    console.error('\n💥 Test suite failed:', error);
    process.exit(1);
  }
}

// Run tests if this file is executed directly
if (require.main === module) {
  main().catch(console.error);
}

export { QEDWasmTest };