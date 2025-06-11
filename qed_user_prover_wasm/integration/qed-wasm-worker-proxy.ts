/**
 * QED WASM Worker Proxy
 * 
 * This class provides a proxy interface to communicate with the QED WASM Web Worker,
 * allowing the main thread to offload intensive computations to a background thread.
 */

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

/**
 * Proxy class for communicating with the QED WASM Web Worker
 */
export class QEDWasmWorkerProxy {
  private worker: Worker | null = null;
  private messageId = 0;
  private pendingMessages = new Map<string, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
  }>();
  private initialized = false;

  constructor(private workerScriptPath: string) {}

  /**
   * Initialize the worker and WASM module
   */
  async initialize(wasmPath?: string): Promise<void> {
    if (this.initialized) {
      return;
    }

    // Create the worker
    this.worker = new Worker(this.workerScriptPath, { type: 'module' });
    
    // Set up message handling
    this.worker.addEventListener('message', this.handleWorkerMessage.bind(this));
    this.worker.addEventListener('error', this.handleWorkerError.bind(this));

    // Initialize the WASM module in the worker
    await this.sendMessage({ type: 'init', wasmPath });
    
    this.initialized = true;
  }

  /**
   * Call a method on the WASM instance in the worker
   */
  async callMethod(method: string, ...args: any[]): Promise<any> {
    if (!this.initialized) {
      throw new Error('Worker not initialized. Call initialize() first.');
    }

    return await this.sendMessage({ type: 'call', method, args });
  }

  /**
   * Dispose of the worker and cleanup resources
   */
  async dispose(): Promise<void> {
    if (!this.worker) {
      return;
    }

    try {
      await this.sendMessage({ type: 'dispose' });
    } catch (error) {
      console.warn('Error disposing worker:', error);
    }

    this.worker.terminate();
    this.worker = null;
    this.initialized = false;
    
    // Reject all pending messages
    for (const { reject } of this.pendingMessages.values()) {
      reject(new Error('Worker terminated'));
    }
    this.pendingMessages.clear();
  }

  /**
   * Send a message to the worker and wait for response
   */
  private async sendMessage(message: Omit<WorkerMessage, 'id'>): Promise<any> {
    if (!this.worker) {
      throw new Error('Worker not available');
    }

    const id = (++this.messageId).toString();
    const fullMessage: WorkerMessage = { id, ...message };

    return new Promise((resolve, reject) => {
      this.pendingMessages.set(id, { resolve, reject });
      
      // Set a timeout for the message
      const timeout = setTimeout(() => {
        this.pendingMessages.delete(id);
        reject(new Error(`Worker message timeout for method: ${message.method || message.type}`));
      }, 30000); // 30 second timeout

      // Store timeout with the message so we can clear it
      const originalResolve = resolve;
      const originalReject = reject;
      
      this.pendingMessages.set(id, {
        resolve: (value) => {
          clearTimeout(timeout);
          originalResolve(value);
        },
        reject: (error) => {
          clearTimeout(timeout);
          originalReject(error);
        },
      });

      this.worker!.postMessage(fullMessage);
    });
  }

  /**
   * Handle messages from the worker
   */
  private handleWorkerMessage(event: MessageEvent<WorkerResponse>): void {
    const { id, type, result, error } = event.data;
    
    const pending = this.pendingMessages.get(id);
    if (!pending) {
      console.warn('Received response for unknown message ID:', id);
      return;
    }

    this.pendingMessages.delete(id);

    if (type === 'success') {
      pending.resolve(result);
    } else {
      pending.reject(new Error(error || 'Unknown worker error'));
    }
  }

  /**
   * Handle worker errors
   */
  private handleWorkerError(event: ErrorEvent): void {
    console.error('Worker error:', event.error);
    
    // Reject all pending messages
    for (const { reject } of this.pendingMessages.values()) {
      reject(new Error(`Worker error: ${event.error?.message || 'Unknown error'}`));
    }
    this.pendingMessages.clear();
  }

  /**
   * Check if the worker is initialized
   */
  isInitialized(): boolean {
    return this.initialized;
  }

  /**
   * Get the number of pending messages
   */
  getPendingMessageCount(): number {
    return this.pendingMessages.size;
  }
}

/**
 * QED WASM Provider using Web Worker for background processing
 */
export class QEDWasmWorkerProvider {
  private workerProxy: QEDWasmWorkerProxy;

  constructor(workerScriptPath: string) {
    this.workerProxy = new QEDWasmWorkerProxy(workerScriptPath);
  }

  async initialize(wasmPath?: string): Promise<void> {
    await this.workerProxy.initialize(wasmPath);
  }

  async ping(): Promise<string> {
    return await this.workerProxy.callMethod('ping');
  }

  async startSession(): Promise<string> {
    const result = await this.workerProxy.callMethod('start_session');
    return JSON.parse(result).session_id;
  }

  async proveContractCall(
    contractAddress: string,
    functionName: string,
    args: string[],
    circuitDef: any
  ): Promise<any> {
    const result = await this.workerProxy.callMethod(
      'prove_contract_call',
      contractAddress,
      functionName,
      JSON.stringify(args),
      JSON.stringify(circuitDef)
    );
    return JSON.parse(result);
  }

  async proveContractCalls(calls: any[]): Promise<any> {
    const result = await this.workerProxy.callMethod(
      'prove_contract_calls',
      JSON.stringify(calls)
    );
    return JSON.parse(result);
  }

  async signAndSubmit(proof: any, transactionData: string): Promise<string> {
    return await this.workerProxy.callMethod(
      'sign_and_submit',
      JSON.stringify(proof),
      transactionData
    );
  }

  async registerUser(userId: string, publicKey: string): Promise<boolean> {
    const result = await this.workerProxy.callMethod('register_user', userId, publicKey);
    return JSON.parse(result);
  }

  async addUser(userId: string, privateKey: string): Promise<boolean> {
    const result = await this.workerProxy.callMethod('add_user', userId, privateKey);
    return JSON.parse(result);
  }

  async switchUser(userId: string): Promise<boolean> {
    const result = await this.workerProxy.callMethod('switch_user', userId);
    return JSON.parse(result);
  }

  async getZKPublicKey(): Promise<string> {
    return await this.workerProxy.callMethod('get_zk_public_key');
  }

  async getRandomKeypair(): Promise<{ publicKey: string; privateKey: string }> {
    const result = await this.workerProxy.callMethod('get_random_keypair');
    const keypair = JSON.parse(result);
    return {
      publicKey: keypair.public_key,
      privateKey: keypair.private_key,
    };
  }

  async deployContract(contractCode: any, constructorArgs: string[]): Promise<string> {
    return await this.workerProxy.callMethod(
      'deploy_contract',
      JSON.stringify(contractCode),
      JSON.stringify(constructorArgs)
    );
  }

  async getDeployContractCmd(contractCode: any, constructorArgs: string[]): Promise<string> {
    return await this.workerProxy.callMethod(
      'get_deploy_contract_cmd',
      JSON.stringify(contractCode),
      JSON.stringify(constructorArgs)
    );
  }

  async getSigHash(message: string): Promise<string> {
    const result = await this.workerProxy.callMethod('get_sighash', message);
    const sigHashInfo = JSON.parse(result);
    return sigHashInfo.hash;
  }

  async getZKSignature(message: string): Promise<string> {
    const result = await this.workerProxy.callMethod('get_zk_signature', message);
    const zkSignature = JSON.parse(result);
    return zkSignature.signature;
  }

  async getEndCapProof(input: any): Promise<any> {
    const result = await this.workerProxy.callMethod('get_end_cap_proof', JSON.stringify(input));
    return JSON.parse(result);
  }

  async getUserECInput(userId: string): Promise<string> {
    return await this.workerProxy.callMethod('get_user_ec_input', userId);
  }

  async getResult(resultId: string): Promise<string> {
    return await this.workerProxy.callMethod('get_result', resultId);
  }

  async dispose(): Promise<void> {
    await this.workerProxy.dispose();
  }

  isInitialized(): boolean {
    return this.workerProxy.isInitialized();
  }

  getPendingOperationCount(): number {
    return this.workerProxy.getPendingMessageCount();
  }
}