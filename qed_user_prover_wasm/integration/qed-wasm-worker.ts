/**
 * QED WASM Web Worker
 * 
 * This worker runs the QED User Prover WASM module in a separate thread
 * to prevent blocking the main UI thread during intensive computations.
 */

import init, { QEDUserProverWasm } from '../pkg/qed_user_prover_wasm';

// Worker message types
interface WorkerMessage {
  id: string;
  type: 'init' | 'call' | 'dispose';
  method?: string;
  args?: any[];
  wasmPath?: string;
}

interface WorkerResponse {
  id: string;
  type: 'success' | 'error';
  result?: any;
  error?: string;
}

class QEDWasmWorker {
  private wasmInstance: QEDUserProverWasm | null = null;
  private initialized = false;

  constructor() {
    // Listen for messages from the main thread
    self.addEventListener('message', this.handleMessage.bind(this));
  }

  private async handleMessage(event: MessageEvent<WorkerMessage>) {
    const { id, type, method, args, wasmPath } = event.data;

    try {
      switch (type) {
        case 'init':
          await this.initialize(wasmPath);
          this.postResponse({ id, type: 'success', result: 'initialized' });
          break;

        case 'call':
          if (!this.initialized || !this.wasmInstance) {
            throw new Error('WASM instance not initialized');
          }
          
          const result = await this.callMethod(method!, args || []);
          this.postResponse({ id, type: 'success', result });
          break;

        case 'dispose':
          this.dispose();
          this.postResponse({ id, type: 'success', result: 'disposed' });
          break;

        default:
          throw new Error(`Unknown message type: ${type}`);
      }
    } catch (error) {
      this.postResponse({
        id,
        type: 'error',
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  private async initialize(wasmPath?: string): Promise<void> {
    if (this.initialized) {
      return;
    }

    // Initialize the WASM module
    await init(wasmPath);
    
    // Create WASM instance
    this.wasmInstance = new QEDUserProverWasm();
    await this.wasmInstance.init();
    
    this.initialized = true;
  }

  private async callMethod(method: string, args: any[]): Promise<any> {
    if (!this.wasmInstance) {
      throw new Error('WASM instance not available');
    }

    // Map method names to WASM instance methods
    switch (method) {
      case 'ping':
        return await this.wasmInstance.ping();
      
      case 'start_session':
        return await this.wasmInstance.start_session();
      
      case 'prove_contract_call':
        return await this.wasmInstance.prove_contract_call(
          args[0], args[1], args[2], args[3]
        );
      
      case 'prove_contract_calls':
        return await this.wasmInstance.prove_contract_calls(args[0]);
      
      case 'sign_and_submit':
        return await this.wasmInstance.sign_and_submit(args[0], args[1]);
      
      case 'register_user':
        return await this.wasmInstance.register_user(args[0], args[1]);
      
      case 'add_user':
        return await this.wasmInstance.add_user(args[0], args[1]);
      
      case 'switch_user':
        return await this.wasmInstance.switch_user(args[0]);
      
      case 'get_zk_public_key':
        return await this.wasmInstance.get_zk_public_key();
      
      case 'get_random_keypair':
        return await this.wasmInstance.get_random_keypair();
      
      case 'deploy_contract':
        return await this.wasmInstance.deploy_contract(args[0], args[1]);
      
      case 'get_deploy_contract_cmd':
        return await this.wasmInstance.get_deploy_contract_cmd(args[0], args[1]);
      
      case 'get_sighash':
        return await this.wasmInstance.get_sighash(args[0]);
      
      case 'get_zk_signature':
        return await this.wasmInstance.get_zk_signature(args[0]);
      
      case 'get_end_cap_proof':
        return await this.wasmInstance.get_end_cap_proof(args[0]);
      
      case 'get_user_ec_input':
        return await this.wasmInstance.get_user_ec_input(args[0]);
      
      case 'get_result':
        return await this.wasmInstance.get_result(args[0]);
      
      default:
        throw new Error(`Unknown method: ${method}`);
    }
  }

  private postResponse(response: WorkerResponse): void {
    self.postMessage(response);
  }

  private dispose(): void {
    this.wasmInstance = null;
    this.initialized = false;
  }
}

// Create and start the worker
new QEDWasmWorker();