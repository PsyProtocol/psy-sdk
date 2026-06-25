import { Felt, PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import { QBCDeployContract, ZKPublicKeyInfo, ContractCallArgs, WalletKeyPair, GUTAStats } from "../types";
interface DPNAssertEqInfoIndexed {
    left: bigint;
    right: bigint;
    message: string;
}
interface DPNIndexedVarDef {
    data_type: number;
    index: number;
    op_type: number;
    inputs: bigint[];
}
interface DPNStateCmd {
    type: number;
    condition: bigint;
    [key: string]: any;
}
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
interface ContractCodeDefinition {
    state_tree_height: number;
    functions: ContractFunctionCodeDefinition[];
}
interface ContractFunctionCodeDefinition {
    method_id: number;
    num_inputs: number;
    num_outputs: number;
    vm_type: number;
    code: U8Bytes;
}
interface Proof {
    wires_cap: any[];
    plonk_zs_partial_products_cap: any[];
    quotient_polys_cap: any[];
    openings: any;
    opening_proof: any;
}
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
    random0: string;
    random1: string;
    note_proof_fingerprint?: [string, string, string, string];
    note_verifier_data?: AltVerifierOnlyCircuitData;
    shield_address?: string;
}
export interface ShieldDepositClaimRaw {
    nullifier: [string, string, string, string];
    note_secret_hash: [string, string, string, string];
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
export type ClaimBatchItem = {
    type: "public";
    data: ContractCallArgs;
} | {
    type: "private_transfer";
    data: {
        contract_id: string;
        claim: PrivateTransferClaimRaw;
    };
} | {
    type: "claim_shield_deposit";
    data: ShieldDepositClaimRaw;
};
interface SubmitUserEndCapNonProofCoreInput {
    checkpoint_id: bigint;
    stats: any;
    state_transition: any;
    new_user_leaf: any;
}
interface PsyContractStateUpdateHistory {
    contract_id: bigint;
    updates: any[];
}
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
export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | {
    [key: string]: JsonValue;
};
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
export interface ProveTxTraceResumableJson {
    generated: GeneratedTxTraceJson;
    proved: ProvedTxResultJson | null;
    error: string | null;
    status: "submitted" | "failed";
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
export interface SimulatedTxJson {
    generated: GeneratedTxTraceJson;
    metadata: TxMetadata;
}
export declare enum SignType {
    ZKSign = "zk",
    SECP256K1Sign = "secp256k1",
    SoftwareDefinedDPNSign = "software-defined-dpn",
    SoftwareDefinedPlonky2Sign = "software-defined-plonky2",
    SDKKeySign = "sdk-key"
}
interface IPsyUserProverProvider {
    execContractCall(pk_hash: string, callData: ContractCallData): Promise<string>;
    execContractCallWithTrace(pk_hash: string, callData: ContractCallData): Promise<TxMetadata>;
    startSession(pk_hash: string): Promise<string>;
    proveContractCall(pk_hash: string, contractCallArg: ContractCallArgs): Promise<string>;
    proveContractCalls(pk_hash: string, contractCallArgs: ContractCallArgs[]): Promise<string>;
    signAndSubmit(pk_hash: string, signData?: SignData): Promise<string>;
    generateTxTrace(pk_hash: string, callData: ContractCallData, localId?: string | null): Promise<GeneratedTxTraceJson>;
    simulateContractCall(pk_hash: string, callData: ContractCallData, localId?: string | null): Promise<GeneratedTxTraceJson>;
    proveTxTrace(pk_hash: string, envelopeJson: string | GeneratedTxTraceJson): Promise<string>;
    proveTxTraceResumable(pk_hash: string, envelopeJson: string | GeneratedTxTraceJson): Promise<ProveTxTraceResumableJson>;
    generateBatchClaimTxTrace(pk_hash: string, claims: ClaimBatchItem[], localId?: string | null): Promise<GeneratedTxTraceJson>;
    batchClaim(pk_hash: string, claims: ClaimBatchItem[], localId?: string | null): Promise<string>;
    claimBatchWithTrace(pk_hash: string, claims: ClaimBatchItem[]): Promise<TxMetadata>;
    claimBatch(pk_hash: string, claims: ClaimBatchItem[]): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
    registerUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    addUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo>;
    getRandomKeypair(): Promise<WalletKeyPair>;
    deployContract(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getDeployContractCmd(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract>;
    ping(message: string): Promise<string>;
    getResult(id: QHashOut): Promise<U8Bytes>;
}
export type { ContractCallArgs, WalletKeyPair, DPNAssertEqInfoIndexed, DPNIndexedVarDef, DPNStateCmd, DPNFunctionCircuitDefinition, ContractCodeDefinition, ContractFunctionCodeDefinition, QBCDeployContract, Proof, ProofWithPublicInputs, AltVerifierOnlyCircuitData, SubmitUserEndCapNonProofCoreInput, PsyContractStateUpdateHistory, SubmitUserEndCapNonProofInput, IPsyUserProverProvider, };
//# sourceMappingURL=types.d.ts.map