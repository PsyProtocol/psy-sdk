import { Felt, PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import { QBCDeployContract, ZKPublicKeyInfo, ContractCallArgs, WalletKeyPair } from "../types";

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
    code: U8Bytes;
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

interface AltVerifierOnlyCircuitData {
    constants_sigmas_cap: QHashOut[];
    circuit_digest: QHashOut;
}

export interface PrivateTransferClaimRaw {
    note_proof_bincode_b64: string;
    nullifier: [string, string, string, string];
    owner: [string, string, string, string];
    amount: string;
    user_tree_root: [string, string, string, string];
    checkpoint_id: string;
    note_root_slot: string;
    note_proof_fingerprint?: [string, string, string, string];
    random0: string;
    random1: string;
    shield_address?: string;
}

interface ShieldDepositClaimRawBase {
    user_id?: string;
    nullifier: [string, string, string, string];
    token_address_u32x8: [string, string, string, string, string, string, string, string];
    l2_token_contract_id: [string, string, string, string, string, string, string, string];
    amount_u32x8: [string, string, string, string, string, string, string, string];
    source_chain_index: string;
    deposit_index: string;
    deposit_root: [string, string, string, string];
    deposit_siblings: [string, string, string, string][];
    random0: string;
    random1: string;
    contract_id: string;
    shield_address?: string;
}

export type ShieldDepositClaimRaw = ShieldDepositClaimRawBase & {
    note_secret: [string, string, string, string];
};

export type ClaimBatchItem =
    | { type: "public"; data: ContractCallArgs }
    | { type: "private_transfer"; data: { contract_id: string; claim: PrivateTransferClaimRaw } }
    | { type: "claim_shield_deposit"; data: ShieldDepositClaimRaw };

// Core input for SubmitUserEndCapNonProofInput
interface SubmitUserEndCapNonProofCoreInput {
    checkpoint_id: bigint;
    stats: any; // GUTAStats
    state_transition: any; // UPSEndCapResultCompact
    new_user_leaf: any; // PsyUserLeaf
}

// Contract state update history
interface PsyContractStateUpdateHistory {
    contract_id: bigint;
    updates: any[]; // Array of contract state updates
}

// SubmitUserEndCapNonProofInput based on Rust definition
interface SubmitUserEndCapNonProofInput {
    core: SubmitUserEndCapNonProofCoreInput;
    contract_state_updates: PsyContractStateUpdateHistory[];
}

export interface SignData {
    inputs: number[];
}

export interface BridgeWithdrawalWitnessInput {
    withdrawal_root: string;
    recipient: number[];
    token: number[];
    amount: number[];
    nonce: number;
    dest_chain_id: number;
    leaf_index: number;
    bridge_user_id: number;
    siblings: string[];
}

export interface BridgeWithdrawalGroth16Proof {
    solidity_proof: string[];
    public_inputs: number[];
}

export interface BridgeDepositLeafInput {
    depositor: number[];
    l2_recipient: number[];
    token: number[];
    l2_token_contract_id: number[];
    amount: number[];
    chain_index: number;
    nonce: number;
}

export interface BridgeDepositBatchWitnessInput {
    from_index: number;
    bridge_user_id: number;
    old_frontier: string[];
    deposits: BridgeDepositLeafInput[];
}

export interface BridgeDepositBatchGroth16Proof {
    solidity_proof: string[];
    public_inputs: number[];
}

export interface ContractCallData {
    contract_calls: ContractCallArgs[];
    software_defined_call: SignData;
}

export enum SignType {
    ZKSign = "zk",
    SECP256K1Sign = "secp256k1",
    SoftwareDefinedDPNSign = "software-defined-dpn",
    SoftwareDefinedPlonky2Sign = "software-defined-plonky2",
    SDKKeySign = "sdk-key"
}

interface IPsyUserProverProvider {
    // Local proving operations
    execContractCall(pk_hash: string, callData: ContractCallData): Promise<string>;
    startSession(pk_hash: string): Promise<string>;
    proveContractCall(pk_hash: string, contractCallArg: ContractCallArgs): Promise<string>;
    proveContractCalls(pk_hash: string, contractCallArgs: ContractCallArgs[]): Promise<string>;
    signAndSubmit(pk_hash: string, signData?: SignData): Promise<string>;
    claimBatch(pk_hash: string, claims: ClaimBatchItem[]): Promise<string>;

    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;

    // User operations
    registerUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    addUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    // switchUser(pkHash: PublicKey): Promise<void>;
    getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo>;
    getRandomKeypair(): Promise<WalletKeyPair>;

    // Contract deployment
    deployContract(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getDeployContractCmd(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract>;

    // Signing and submission
    // getSigHash(networkMagic: bigint): Promise<QHashOut>;
    // getZKSignature(sighash: QHashOut): Promise<ProofWithPublicInputs>;
    // getEndCapProof(signatureProof: ProofWithPublicInputs): Promise<ProofWithPublicInputs>;
    // getUserECInput(): Promise<SubmitUserEndCapNonProofInput>;

    // Utility methods
    ping(message: string): Promise<string>;
    getResult(id: QHashOut): Promise<U8Bytes>;
}

export type {
    ContractCallArgs,
    WalletKeyPair,
    DPNAssertEqInfoIndexed,
    DPNIndexedVarDef,
    DPNStateCmd,
    DPNFunctionCircuitDefinition,
    ContractCodeDefinition,
    ContractFunctionCodeDefinition,
    QBCDeployContract,
    Proof,
    ProofWithPublicInputs,
    AltVerifierOnlyCircuitData,
    SubmitUserEndCapNonProofCoreInput,
    PsyContractStateUpdateHistory,
    SubmitUserEndCapNonProofInput,
    IPsyUserProverProvider,
};
