/**
 * QED WASM Provider for TypeScript SDK Integration
 * 
 * This module provides a TypeScript wrapper for the QED User Prover WASM module,
 * implementing the IQEDUserProverProvider interface for seamless integration with qed-ts-sdk.
 */

import init, {
  QEDUserProverWasm,
  ProofWithPublicInputs,
  SessionInfo,
  UserInfo,
  KeypairInfo,
  DeployContractCmd,
  SigHashInfo,
  ZKSignature,
  AsyncResult,
  DPNFunctionCircuitDefinition,
  ContractCodeDefinition,
  SubmitUserEndCapNonProofInput,
} from '../pkg/qed_user_prover_wasm';

// Import types from qed-ts-sdk
import {
  IQEDUserProverProvider,
  QEDUserProverRPCCommand,
  ProofWithPublicInputs as SDKProofWithPublicInputs,
  DPNFunctionCircuitDefinition as SDKDPNFunctionCircuitDefinition,
  ContractCodeDefinition as SDKContractCodeDefinition,
  SubmitUserEndCapNonProofInput as SDKSubmitUserEndCapNonProofInput,
} from 'qed-ts-sdk';

/**
 * Configuration options for the WASM provider
 */
export interface QEDWasmProviderConfig {
  /** Path to the WASM binary file */
  wasmPath?: string;
  /** Enable debug logging */
  debug?: boolean;
  /** Worker thread configuration */
  useWorker?: boolean;
  /** Memory limit for WASM module (in MB) */
  memoryLimit?: number;
}

/**
 * QED WASM Provider implementing the IQEDUserProverProvider interface
 */
export class QEDWasmProvider implements IQEDUserProverProvider {
  private wasmInstance: QEDUserProverWasm | null = null;
  private initialized = false;
  private config: QEDWasmProviderConfig;
  private worker: Worker | null = null;

  constructor(config: QEDWasmProviderConfig = {}) {
    this.config = {
      debug: false,
      useWorker: true,
      memoryLimit: 512,
      ...config,
    };
  }

  /**
   * Initialize the WASM module
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    try {
      // Initialize the WASM module
      await init(this.config.wasmPath);
      
      // Create WASM instance
      this.wasmInstance = new QEDUserProverWasm();
      await this.wasmInstance.init();
      
      this.initialized = true;
      
      if (this.config.debug) {
        console.log('QED WASM Provider initialized successfully');
      }
    } catch (error) {
      console.error('Failed to initialize QED WASM Provider:', error);
      throw new Error(`WASM initialization failed: ${error}`);
    }
  }

  /**
   * Ensure the WASM module is initialized
   */
  private ensureInitialized(): void {
    if (!this.initialized || !this.wasmInstance) {
      throw new Error('QED WASM Provider not initialized. Call initialize() first.');
    }
  }

  /**
   * Convert WASM types to SDK types
   */
  private convertProofToSDK(wasmProof: ProofWithPublicInputs): SDKProofWithPublicInputs {
    return {
      proof: {
        wires_cap: wasmProof.proof.wires_cap,
        plonk_zs_partial_products_cap: wasmProof.proof.plonk_zs_partial_products_cap,
        quotient_polys_cap: wasmProof.proof.quotient_polys_cap,
        openings: wasmProof.proof.openings,
        opening_proof: wasmProof.proof.opening_proof,
      },
      public_inputs: wasmProof.public_inputs,
    };
  }

  // Implement IQEDUserProverProvider interface

  async ping(): Promise<string> {
    this.ensureInitialized();
    return await this.wasmInstance!.ping();
  }

  async startSession(): Promise<string> {
    this.ensureInitialized();
    const sessionInfo = await this.wasmInstance!.start_session();
    return JSON.parse(sessionInfo).session_id;
  }

  async proveContractCall(
    contractAddress: string,
    functionName: string,
    args: string[],
    circuitDef: SDKDPNFunctionCircuitDefinition
  ): Promise<SDKProofWithPublicInputs> {
    this.ensureInitialized();
    
    const wasmCircuitDef: DPNFunctionCircuitDefinition = {
      function_name: circuitDef.function_name,
      input_vars: circuitDef.input_vars,
      output_vars: circuitDef.output_vars,
      assert_eq_infos: circuitDef.assert_eq_infos,
      state_cmds: circuitDef.state_cmds,
    };
    
    const result = await this.wasmInstance!.prove_contract_call(
      contractAddress,
      functionName,
      JSON.stringify(args),
      JSON.stringify(wasmCircuitDef)
    );
    
    const wasmProof: ProofWithPublicInputs = JSON.parse(result);
    return this.convertProofToSDK(wasmProof);
  }

  async proveContractCalls(
    calls: Array<{
      contractAddress: string;
      functionName: string;
      args: string[];
      circuitDef: SDKDPNFunctionCircuitDefinition;
    }>
  ): Promise<SDKProofWithPublicInputs> {
    this.ensureInitialized();
    
    const wasmCalls = calls.map(call => ({
      contract_address: call.contractAddress,
      function_name: call.functionName,
      args: call.args,
      circuit_def: {
        function_name: call.circuitDef.function_name,
        input_vars: call.circuitDef.input_vars,
        output_vars: call.circuitDef.output_vars,
        assert_eq_infos: call.circuitDef.assert_eq_infos,
        state_cmds: call.circuitDef.state_cmds,
      },
    }));
    
    const result = await this.wasmInstance!.prove_contract_calls(JSON.stringify(wasmCalls));
    const wasmProof: ProofWithPublicInputs = JSON.parse(result);
    return this.convertProofToSDK(wasmProof);
  }

  async signAndSubmit(
    proof: SDKProofWithPublicInputs,
    transactionData: string
  ): Promise<string> {
    this.ensureInitialized();
    
    const wasmProof: ProofWithPublicInputs = {
      proof: {
        wires_cap: proof.proof.wires_cap,
        plonk_zs_partial_products_cap: proof.proof.plonk_zs_partial_products_cap,
        quotient_polys_cap: proof.proof.quotient_polys_cap,
        openings: proof.proof.openings,
        opening_proof: proof.proof.opening_proof,
      },
      public_inputs: proof.public_inputs,
    };
    
    return await this.wasmInstance!.sign_and_submit(
      JSON.stringify(wasmProof),
      transactionData
    );
  }

  async registerUser(userId: string, publicKey: string): Promise<boolean> {
    this.ensureInitialized();
    const result = await this.wasmInstance!.register_user(userId, publicKey);
    return JSON.parse(result);
  }

  async addUser(userId: string, privateKey: string): Promise<boolean> {
    this.ensureInitialized();
    const result = await this.wasmInstance!.add_user(userId, privateKey);
    return JSON.parse(result);
  }

  async switchUser(userId: string): Promise<boolean> {
    this.ensureInitialized();
    const result = await this.wasmInstance!.switch_user(userId);
    return JSON.parse(result);
  }

  async getZKPublicKey(): Promise<string> {
    this.ensureInitialized();
    return await this.wasmInstance!.get_zk_public_key();
  }

  async getRandomKeypair(): Promise<{ publicKey: string; privateKey: string }> {
    this.ensureInitialized();
    const result = await this.wasmInstance!.get_random_keypair();
    const keypair: KeypairInfo = JSON.parse(result);
    return {
      publicKey: keypair.public_key,
      privateKey: keypair.private_key,
    };
  }

  async deployContract(
    contractCode: SDKContractCodeDefinition,
    constructorArgs: string[]
  ): Promise<string> {
    this.ensureInitialized();
    
    const wasmContractCode: ContractCodeDefinition = {
      contract_name: contractCode.contract_name,
      functions: contractCode.functions.map(func => ({
        function_name: func.function_name,
        circuit_def: {
          function_name: func.circuit_def.function_name,
          input_vars: func.circuit_def.input_vars,
          output_vars: func.circuit_def.output_vars,
          assert_eq_infos: func.circuit_def.assert_eq_infos,
          state_cmds: func.circuit_def.state_cmds,
        },
      })),
    };
    
    return await this.wasmInstance!.deploy_contract(
      JSON.stringify(wasmContractCode),
      JSON.stringify(constructorArgs)
    );
  }

  async getDeployContractCmd(
    contractCode: SDKContractCodeDefinition,
    constructorArgs: string[]
  ): Promise<string> {
    this.ensureInitialized();
    
    const wasmContractCode: ContractCodeDefinition = {
      contract_name: contractCode.contract_name,
      functions: contractCode.functions.map(func => ({
        function_name: func.function_name,
        circuit_def: {
          function_name: func.circuit_def.function_name,
          input_vars: func.circuit_def.input_vars,
          output_vars: func.circuit_def.output_vars,
          assert_eq_infos: func.circuit_def.assert_eq_infos,
          state_cmds: func.circuit_def.state_cmds,
        },
      })),
    };
    
    return await this.wasmInstance!.get_deploy_contract_cmd(
      JSON.stringify(wasmContractCode),
      JSON.stringify(constructorArgs)
    );
  }

  async getSigHash(message: string): Promise<string> {
    this.ensureInitialized();
    const result = await this.wasmInstance!.get_sighash(message);
    const sigHashInfo: SigHashInfo = JSON.parse(result);
    return sigHashInfo.hash;
  }

  async getZKSignature(message: string): Promise<string> {
    this.ensureInitialized();
    const result = await this.wasmInstance!.get_zk_signature(message);
    const zkSignature: ZKSignature = JSON.parse(result);
    return zkSignature.signature;
  }

  async getEndCapProof(
    input: SDKSubmitUserEndCapNonProofInput
  ): Promise<SDKProofWithPublicInputs> {
    this.ensureInitialized();
    
    const wasmInput: SubmitUserEndCapNonProofInput = {
      core_input: {
        user_id: input.core_input.user_id,
        contract_address: input.core_input.contract_address,
        function_name: input.core_input.function_name,
        inputs: input.core_input.inputs,
        timestamp: input.core_input.timestamp,
      },
      state_history: {
        contract_address: input.state_history.contract_address,
        updates: input.state_history.updates,
      },
      additional_data: input.additional_data,
    };
    
    const result = await this.wasmInstance!.get_end_cap_proof(JSON.stringify(wasmInput));
    const wasmProof: ProofWithPublicInputs = JSON.parse(result);
    return this.convertProofToSDK(wasmProof);
  }

  async getUserECInput(userId: string): Promise<string> {
    this.ensureInitialized();
    return await this.wasmInstance!.get_user_ec_input(userId);
  }

  async getResult(resultId: string): Promise<string> {
    this.ensureInitialized();
    return await this.wasmInstance!.get_result(resultId);
  }

  /**
   * Cleanup resources
   */
  async dispose(): Promise<void> {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    
    if (this.wasmInstance) {
      // Cleanup WASM instance if needed
      this.wasmInstance = null;
    }
    
    this.initialized = false;
  }
}

/**
 * Factory function to create and initialize a QED WASM Provider
 */
export async function createQEDWasmProvider(
  config: QEDWasmProviderConfig = {}
): Promise<QEDWasmProvider> {
  const provider = new QEDWasmProvider(config);
  await provider.initialize();
  return provider;
}

/**
 * Export types for external use
 */
export {
  QEDUserProverRPCCommand,
  type SDKProofWithPublicInputs as ProofWithPublicInputs,
  type SDKDPNFunctionCircuitDefinition as DPNFunctionCircuitDefinition,
  type SDKContractCodeDefinition as ContractCodeDefinition,
  type SDKSubmitUserEndCapNonProofInput as SubmitUserEndCapNonProofInput,
};