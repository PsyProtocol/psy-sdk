import { Felt, PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import { QBCDeployContractV2, ZKPublicKeyInfo, ContractCallArgs, WalletKeyPair, GUTAStats } from "../types";

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
    note_proof?: Uint8Array;
    note_proof_bincode_b64?: string;
    nullifier: [string, string, string, string];
    owner: [string, string, string, string];
    amount: string;
    user_tree_root: [string, string, string, string];
    checkpoint_id: string;
    note_root_slot: string;
    random0: string;
    random1: string;
    note_proof_fingerprint?: [string, string, string, string];
    note_verifier_data?: AltVerifierOnlyCircuitData;
    shield_address?: string;
    /** Canonical producer→services→wallet contract identity for the note's
     *  token contract. Decimal string; bound to the private-note inclusion
     *  proof's public inputs. Clean cutover: every PrivateTransferClaimRaw
     *  MUST carry this — no optional fallback / default-to-0. Wallet builders
     *  always populate it from the validated note-proof envelope. */
    token_contract_id: string;
}

interface DepositInclusionClaimRawBase {
    user_id?: string;
    nullifier_hash: [string, string, string, string];
    note_commitment: [string, string, string, string];
    token_address_u32x8: [string, string, string, string, string, string, string, string];
    l2_token_contract_id: [string, string, string, string, string, string, string, string];
    amount_u32x8: [string, string, string, string, string, string, string, string];
    source_chain_index: string;
    deposit_index: string;
    deposit_root: [string, string, string, string];
    deposit_proof_bincode_b64: string;
    deposit_proof_fingerprint?: [string, string, string, string];
    random0: string;
    random1: string;
    contract_id: string;
    shield_address?: string;
}

export type DepositInclusionClaimRaw = DepositInclusionClaimRawBase;

export type ClaimBatchItem =
    | { type: "public"; data: ContractCallArgs }
    | { type: "private_transfer"; data: { contract_id: string; claim: PrivateTransferClaimRaw } }
    | { type: "claim_shield_deposit"; data: DepositInclusionClaimRaw };

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
    sender_user_id: number;
    recipient: number[];
    token: number[];
    amount: number[];
    nonce: number[];
    destination_chain_index: number;
    leaf_index: number;
    bridge_user_id: number;
    siblings: string[];
}

export interface BridgeWithdrawalBatchWitnessInput {
    bridge_user_id: number;
    withdrawals: BridgeWithdrawalWitnessInput[];
}

export interface BridgeWithdrawalGroth16Proof {
    solidity_proof: string[];
    public_inputs: number[];
    slot_data: number[];
}

export interface BridgeWithdrawalBatchGroth16Proof {
    solidity_proof: string[];
    public_inputs: number[];
    slot_data: number[];
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

/** View-only call request: contract calls only, no software-defined / fee path. */
export interface ViewCallData {
    contract_calls: ContractCallArgs[];
}

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

export interface TracePayload {
    encoding: string;
    payload: string;
}

export interface GeneratedTxTraceJson {
    user_id: string;
    pk_hash: string;
    sig_hash: string;
    tx_hash: string;
    call_data: JsonValue;
    tx_count: number;
    trace: TracePayload;
}

export interface ProvedTxResultJson {
    sig_hash: string;
    tx_hash: string;
    checkpoint_id: Felt | null;
    status: string;
}

export type ProvingStateBlobJson = Uint8Array;
export type ProvingProofBlobJson = Uint8Array;

export type TraceStepProgressJson =
    | {
          done: false;
          state: ProvingStateBlobJson;
          proofs: ProvingProofBlobJson[];
      }
    | {
          done: true;
          tx_hash: string;
      }
    | {
          done: false;
          error: string;
          state?: undefined;
          proofs?: undefined;
          tx_hash?: undefined;
      };

export interface InitStepProvingJson {
    proof_tree_meta: unknown;
    last_step_info: unknown;
    current_header: unknown;
    previous_header: unknown;
    ups_proof: Uint8Array;
}

export interface ProveStepJson {
    cfc_proof: Uint8Array;
    ups_proof: Uint8Array;
    proof_tree_meta: unknown;
    last_step_info: unknown;
    current_header: unknown;
    previous_header: unknown;
}

export interface ProveEndCapProofJson {
    end_cap_proof: Uint8Array;
    tx_hash: string;
}

export type TraceProofScheduleJson = string;
export type TraceProofJobOutputJson = string;

export interface TraceProofJobStepIndices {
    cfc_step_indices: number[];
    external_step_indices: number[];
}

export interface TraceProofConcurrentResult {
    scheduleJson: TraceProofScheduleJson;
    firstWaveOutputJsons: TraceProofJobOutputJson[];
    endcapOutputJson: TraceProofJobOutputJson;
    submitOutputJson: TraceProofJobOutputJson;
}

export interface TxStorageRead {
    user_id: Felt;
    contract_id: Felt;
    slot_index: Felt;
    value: QHashOut;
}

export interface TxStorageWrite {
    user_id: Felt;
    contract_id: Felt;
    slot_index: Felt;
    old_value: QHashOut;
    new_value: QHashOut;
}

export interface TxStorageData {
    reads: TxStorageRead[];
    writes: TxStorageWrite[];
}

export interface ContractCallResultArgs {
    contract_id: Felt;
    method_name: string;
    inputs: Felt[];
    outputs: Felt[];
}

export interface ContractCallResultData {
    contract_calls: ContractCallResultArgs[];
    software_defined_call: SignData;
}

export interface TxEndCapData {
    checkpoint_id: Felt;
    user_id: Felt;
    global_user_tree_height: number;
    start_user_leaf_hash: QHashOut;
    end_user_leaf_hash: QHashOut;
    checkpoint_tree_root_hash: QHashOut;
    stats: GUTAStats;
}

export interface TxProofMetadata {
    storage_data: TxStorageData;
    contract_calls: ContractCallResultArgs[];
}

export interface TxSubmitMetadata {
    tx_hash: QHashOut;
    end_cap_data: TxEndCapData;
    storage_writes: TxStorageWrite[];
}

export interface TxMetadata {
    tx_hash: QHashOut;
    end_cap_data: TxEndCapData;
    contract_call_data: ContractCallResultData;
    storage_data: TxStorageData;
}

export interface SimulatedTxMetadata {
    tx_hash?: QHashOut;
    end_cap_data?: TxEndCapData;
    contract_call_data: ContractCallResultData;
    storage_data: TxStorageData;
}

export interface SimulatedTxJson {
    generated?: GeneratedTxTraceJson;
    metadata: SimulatedTxMetadata;
}

/** Fixed fee-free view result: no generated trace, tx hash, or end-cap fields. */
export interface ViewCallResult {
    checkpoint_id: Felt;
    contract_calls: ContractCallResultArgs[];
    storage_reads: TxStorageRead[];
}

export enum SignType {
    ZKSign = "zk",
    SECP256K1Sign = "secp256k1",
    EthPersonalSECP256K1Sign = "eth-personal-secp256k1",
    SoftwareDefinedDPNSign = "software-defined-dpn",
    SoftwareDefinedPlonky2Sign = "software-defined-plonky2",
    SDKeySign = "sd-key",
    SDKKeySign = "sd-key"
}

interface IPsyUserProverProvider {
    // Local proving operations
    execContractCall(pk_hash: string, callData: ContractCallData): Promise<string>;
    execContractCallWithTrace(pk_hash: string, callData: ContractCallData): Promise<TxMetadata>;
    startSession(pk_hash: string): Promise<string>;
    proveContractCall(pk_hash: string, contractCallArg: ContractCallArgs): Promise<string>;
    proveContractCalls(pk_hash: string, contractCallArgs: ContractCallArgs[]): Promise<string>;
    signAndSubmit(pk_hash: string, signData?: SignData): Promise<string>;
    generateTxTrace(pk_hash: string, callData: ContractCallData, localId?: string | null): Promise<GeneratedTxTraceJson>;
    simulateContractCall(pk_hash: string, callData: ContractCallData, localId?: string | null): Promise<SimulatedTxJson>;
    callView(pk_hash: string, callData: ViewCallData, localId?: string | null): Promise<ViewCallResult>;
    proveUpsStart(pk_hash: string, envelopeJson: string | GeneratedTxTraceJson): Promise<InitStepProvingJson>;
    proveTraceStep(
        pk_hash: string,
        envelopeJson: string | GeneratedTxTraceJson,
        stateBlob?: ProvingStateBlobJson,
        proofs?: ProvingProofBlobJson[],
    ): Promise<TraceStepProgressJson>;
    prepareTraceProofSchedule(envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofScheduleJson>;
    getTraceProofJobStepIndices(envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofJobStepIndices>;
    proveUpsStartJob(pk_hash: string, envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofJobOutputJson>;
    proveCfcJobWithScheduleStep(
        pk_hash: string,
        envelopeJson: string | GeneratedTxTraceJson,
        scheduleJson: TraceProofScheduleJson,
        stepIndex: number,
    ): Promise<TraceProofJobOutputJson>;
    proveExternalProofJob(envelopeJson: string | GeneratedTxTraceJson, stepIndex: number): Promise<TraceProofJobOutputJson>;
    proveZkSignJob(pk_hash: string, envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofJobOutputJson>;
    proveEndcapJobFromOutputJsons(
        pk_hash: string,
        envelopeJson: string | GeneratedTxTraceJson,
        scheduleJson: TraceProofScheduleJson,
        outputJsons: TraceProofJobOutputJson[],
    ): Promise<TraceProofJobOutputJson>;
    submitEndcapJob(envelopeJson: string | GeneratedTxTraceJson, endcapOutputJson: TraceProofJobOutputJson): Promise<TraceProofJobOutputJson>;
    proveTraceJobsConcurrent(pk_hash: string, envelopeJson: string | GeneratedTxTraceJson): Promise<TraceProofConcurrentResult>;

    proveEndCapProof(
        pk_hash: string,
        envelopeJson: string | GeneratedTxTraceJson,
        proofTreeMeta: unknown,
        lastStepInfo: unknown,
        allProofBlobs: Uint8Array[],
        signatureProof: Uint8Array,
    ): Promise<ProveEndCapProofJson>;
    submitEndCap(envelopeJson: string | GeneratedTxTraceJson, endCapProof: Uint8Array): Promise<string>;
    signSighash(pk_hash: string, sighashJson: string, envelopeJson?: string | GeneratedTxTraceJson, currentHeader?: unknown): Promise<Uint8Array>;
    computeSighashFromEnvelope(envelopeJson: string | GeneratedTxTraceJson, currentHeader: unknown): Promise<string>;
    insertExternalProof(
        pk_hash: string,
        envelopeJson: string | GeneratedTxTraceJson,
        proofTreeMeta: unknown,
        lastStepInfo: unknown,
        currentHeader: unknown,
        previousHeader: unknown,
        externalFingerprint: string,
        externalProof: Uint8Array,
    ): Promise<any>;
    generateBatchClaimTxTrace(pk_hash: string, claims: ClaimBatchItem[], localId?: string | null): Promise<GeneratedTxTraceJson>;
    batchClaim(pk_hash: string, claims: ClaimBatchItem[], localId?: string | null): Promise<string>;
    claimBatchWithTrace(pk_hash: string, claims: ClaimBatchItem[]): Promise<TxMetadata>;
    claimBatch(pk_hash: string, claims: ClaimBatchItem[]): Promise<string>;

    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;

    // User operations
    registerUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    addUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    // switchUser(pkHash: PublicKey): Promise<void>;
    getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo>;
    getRandomKeypair(): Promise<WalletKeyPair>;

    // External EIP-191 personal_sign signer (browser / MetaMask flow).
    // The canonical Rust adapter recovers/authenticates the signer from the
    // supplied signature; the SDK performs no cryptography itself.
    ethPersonalRegistrationChallenge(selectedEvmAddressHex: string): Promise<string>;
    registerExternalEthPersonalUser(selectedEvmAddressHex: string, recoveryMessageHex32: string, signatureHex65: string): Promise<PublicKey>;
    injectEthPersonalSignature(expectedPkHash: PublicKey, selectedEvmAddressHex: string, messageHex32: string, signatureHex65: string): Promise<PublicKey>;

    // Contract deployment
    deployContract(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getDeployContractCmd(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[], abi: unknown): Promise<QBCDeployContractV2>;

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
    QBCDeployContractV2,
    Proof,
    ProofWithPublicInputs,
    AltVerifierOnlyCircuitData,
    SubmitUserEndCapNonProofCoreInput,
    PsyContractStateUpdateHistory,
    SubmitUserEndCapNonProofInput,
    IPsyUserProverProvider,
};
