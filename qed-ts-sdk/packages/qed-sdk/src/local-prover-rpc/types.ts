import { Felt, PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";

// Namespace corresponds to "qed" in Rust
enum QEDUserProverRPCCommand {
    StartSession = "qed_start_session",
    ProveContractCall = "qed_prove_contract_call",
    ProveContractCalls = "qed_prove_contract_calls",
    SignAndSubmit = "qed_sign_and_submit",
    RegisterUser = "qed_register_user",
    AddUser = "qed_add_user",
    SwitchUser = "qed_switch_user",
    GetZKPublicKey = "qed_get_zk_public_key",
    GetRandomKeypair = "qed_get_random_keypair",
    DeployContract = "qed_deploy_contract",
    GetDeployContractCmd = "qed_get_deploy_contract_cmd",
    GetSigHash = "qed_get_sighash",
    GetZKSignature = "qed_get_zk_signature",
    GetEndCapProof = "qed_get_end_cap_proof",
    GetUserECInput = "qed_get_user_ec_input",
    Ping = "qed_ping",
    GetResult = "qed_get_result",
}

// Converted from Rust ContractCallArgs
interface ContractCallArgs {
    contract_id: bigint;
    method_name: string;
    inputs: bigint[];
}

// Converted from Rust ZKPublicKeyInfo
interface ZKPublicKeyInfo {
    fingerprint: QHashOut;
    public_key_param: QHashOut;
}

// Converted from Rust WalletKeyPair
interface WalletKeyPair {
    private_key: PrivateKey;
    public_key: ZKPublicKeyInfo;
}

// Assertion for DPN function circuits
interface DPNAssertEqInfoIndexed {
    left: bigint;
    right: bigint;
    message: string;
}

// Variable definition for DPN function circuits
interface DPNIndexedVarDef {
    data_type: number;
    index: number;
    op_type: number;
    inputs: bigint[];
}

// State command for DPN function circuits
interface DPNStateCmd {
    type: number;
    condition: bigint;
    [key: string]: any; // Additional properties based on command type
}

// Complete DPNFunctionCircuitDefinition based on Rust definition
interface DPNFunctionCircuitDefinition {
    name: string;
    method_id: number;
    circuit_inputs: bigint[];
    circuit_outputs: bigint[];
    state_commands: DPNStateCmd[];
    state_command_resolution_indices: number[];
    assertions: DPNAssertEqInfoIndexed[];
    definitions: DPNIndexedVarDef[];
}

// Contract Code Definition for QBCDeployContract
interface ContractCodeDefinition {
    state_tree_height: number;
    functions: ContractFunctionCodeDefinition[];
}

// ContractFunctionCodeDefinition used in contract deployment
interface ContractFunctionCodeDefinition {
    method_id: number;
    num_inputs: number;
    num_outputs: number;
    vm_type: number;
    code: Uint8Array | string;
}

// QBCDeployContract based on Rust definition
interface QBCDeployContract {
    deployer: QHashOut;
    code_definition: ContractCodeDefinition;
    function_whitelist: QHashOut[];
}

// Proof component from plonky2
interface Proof {
    wires_cap: any[];
    plonk_zs_partial_products_cap: any[];
    quotient_polys_cap: any[];
    openings: any;
    opening_proof: any;
}

// ProofWithPublicInputs based on plonky2 definition
interface ProofWithPublicInputs {
    proof: Proof;
    public_inputs: Felt[];
}

// Core input for SubmitUserEndCapNonProofInput
interface SubmitUserEndCapNonProofCoreInput {
    checkpoint_id: bigint;
    stats: any; // GUTAStats
    state_transition: any; // UPSEndCapResultCompact
    new_user_leaf: any; // QEDUserLeaf
}

// Contract state update history
interface QEDContractStateUpdateHistory {
    contract_id: bigint;
    updates: any[]; // Array of contract state updates
}

// SubmitUserEndCapNonProofInput based on Rust definition
interface SubmitUserEndCapNonProofInput {
    core: SubmitUserEndCapNonProofCoreInput;
    contract_state_updates: QEDContractStateUpdateHistory[];
}

interface IQEDUserProverProvider {
    // Local proving operations
    startSession(): Promise<string>;
    proveContractCall(contractCallArg: ContractCallArgs): Promise<string>;
    proveContractCalls(contractCallArgs: ContractCallArgs[]): Promise<string>;
    signAndSubmit(): Promise<string>;

    // User operations
    registerUser(privateKey: PrivateKey): Promise<QHashOut>;
    addUser(privateKey: PrivateKey): Promise<PublicKey>;
    switchUser(pkHash: PublicKey): Promise<void>;
    getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo>;
    getRandomKeypair(): Promise<WalletKeyPair>;

    // Contract deployment
    deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getDeployContractCmd(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract>;

    // Signing and submission
    getSigHash(networkMagic: bigint): Promise<QHashOut>;
    getZKSignature(sighash: QHashOut): Promise<ProofWithPublicInputs>;
    getEndCapProof(signatureProof: ProofWithPublicInputs): Promise<ProofWithPublicInputs>;
    getUserECInput(): Promise<SubmitUserEndCapNonProofInput>;

    // Utility methods
    ping(message: string): Promise<string>;
    getResult(id: QHashOut): Promise<U8Bytes>;
}

export type {
    ContractCallArgs,
    WalletKeyPair,
    ZKPublicKeyInfo,
    QHashOut,
    DPNAssertEqInfoIndexed,
    DPNIndexedVarDef,
    DPNStateCmd,
    DPNFunctionCircuitDefinition,
    ContractCodeDefinition,
    ContractFunctionCodeDefinition,
    QBCDeployContract,
    Proof,
    ProofWithPublicInputs,
    SubmitUserEndCapNonProofCoreInput,
    QEDContractStateUpdateHistory,
    SubmitUserEndCapNonProofInput,
    IQEDUserProverProvider,
};

export { QEDUserProverRPCCommand };
