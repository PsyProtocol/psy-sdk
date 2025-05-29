import { ProofWithPublicInputs } from "../types";

/**
 * QHash output type with elements
 */
export interface QHashOut {
    elements: bigint[];
}

/**
 * Generic Merkle proof core structure
 */
export interface MerkleProofCore<T> {
    root: T;
    value: T;
    index: bigint;
    siblings: T[];
}

/**
 * QED Checkpoint Leaf data structure
 */
export interface QEDCheckpointLeaf {
    checkpoint_id: bigint;
    next_add_withdrawal_id: bigint;
    next_process_withdrawal_id: bigint;
    next_deposit_id: bigint;
    total_deposits_claimed_epoch: bigint;
    next_user_id: bigint;
    end_balance: bigint;
}

/**
 * L2 Block State structure
 */
export interface QEDL2BlockState {
    checkpoint_id: bigint;
    next_add_withdrawal_id: bigint;
    next_process_withdrawal_id: bigint;
    next_deposit_id: bigint;
    total_deposits_claimed_epoch: bigint;
    next_user_id: bigint;
    end_balance: bigint;
}

/**
 * QED User Leaf structure
 */
export interface QEDUserLeaf {
    user_id: bigint;
    balance: bigint;
    nonce: bigint;
    alt_0: bigint;
    alt_1: bigint;
    public_key: QHashOut;
}

/**
 * QED Contract Leaf structure
 */
export interface QEDContractLeaf {
    contract_id: bigint;
    owner_id: bigint;
    code_hash: QHashOut;
    whitelist_root: QHashOut;
    is_core_contract: boolean;
    create_checkpoint_id: bigint;
}

/**
 * QED Contract Code Definition
 */
export interface ContractCodeDefinition {
    contract_id: bigint;
    code_hash: QHashOut;
    code_path: string;
    code_size: bigint;
}

/**
 * QED Checkpoint Global State Roots
 */
export interface QEDCheckpointGlobalStateRoots {
    checkpoint_id: bigint;
    withdrawal_tree_root: QHashOut;
    user_tree_root: QHashOut;
    contract_tree_root: QHashOut;
    deposit_tree_root: QHashOut;
}

/**
 * QED Checkpoint Sync Info Compact
 */
export interface QEDCheckpointSyncInfoCompact {
    checkpoint_id: bigint;
    checkpoint_tree_root: QHashOut;
    checkpoint_leaf_data: QEDCheckpointLeaf;
    global_state_roots: QEDCheckpointGlobalStateRoots;
}

/**
 * Checkpoint Sync Info Full Structure
 */
export interface CheckpointSyncInfo {
    latest_checkpoint_id: bigint;
    description: string | null;
    source_coordinator_edge_id: string | null;
    sync_timestamp: bigint;
    compact: QEDCheckpointSyncInfoCompact;
}

/**
 * ZK Public Key Info
 */
export interface ZKPublicKeyInfo {
    public_key_param: QHashOut;
}

/**
 * QBC Deploy Contract params
 */
export interface QBCDeployContract {
    user_id: bigint;
    code_hash: QHashOut;
    code_path: string;
    code_size: bigint;
    contract_id: bigint | null; // Optional contract ID
    signature_proof: string;
}

/**
 * GUTA Submit Request structure
 */
export interface SubmitGUTAParams {
    input: SubmitGUTARealmResultAPINoProofInput;
    proof: ProofWithPublicInputs;
}

/**
 * GUTA Realm Result API No Proof Input
 */
export interface SubmitGUTARealmResultAPINoProofInput {
    circuit_type: number;
    realm_id: bigint;
    top_line_proof: MerkleProofCore<QHashOut>;
}

/**
 * Response for the latest checkpoint
 */
export interface LatestCheckpointResponse {
    checkpoint_id: bigint;
}

// Request types for RPC methods
export interface QContractLeafDataRPCRequest {
    contract_id: number;
}

export interface QContractLeafDataFRPCRequest {
    contract_id: bigint;
}

export interface QCheckpointLeafDataRPCRequest {
    checkpoint_id: number;
}

export interface QCheckpointLeafDataFRPCRequest {
    checkpoint_id: bigint;
}

export interface QContractCodeDefinitionRPCRequest {
    contract_id: number;
}

export interface QContractCodeDefinitionFRPCRequest {
    contract_id: bigint;
}

export interface QL2BlockStateRPCRequest {
    checkpoint_id: number;
}

export interface QL2BlockStateFRPCRequest {
    checkpoint_id: bigint;
}

export interface QUserRegistrationTreeRootRPCRequest {
    checkpoint_id: number;
}

export interface QUserRegistrationTreeRootFRPCRequest {
    checkpoint_id: bigint;
}

export interface QUserRegistrationTreeLeafHashRPCRequest {
    checkpoint_id: number;
    leaf_index: number;
}

export interface QUserRegistrationTreeLeafHashFRPCRequest {
    checkpoint_id: bigint;
    leaf_index: bigint;
}

export interface QUserRegistrationTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    leaf_index: number;
}

export interface QUserRegistrationTreeMerkleProofFRPCRequest {
    checkpoint_id: bigint;
    leaf_index: bigint;
}

export interface QUserTreeRootRPCRequest {
    checkpoint_id: number;
}

export interface QUserTreeRootFRPCRequest {
    checkpoint_id: bigint;
}

export interface QUserSubTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    root_level: number;
    leaf_level: number;
    leaf_index: number;
}

export interface QUserLeafDataRPCRequest {
    checkpoint_id: number;
    user_id: number;
}

export interface QUserTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    user_id: number;
}

export interface QUserTreeMerkleProofFRPCRequest {
    checkpoint_id: bigint;
    user_id: bigint;
}

export interface QContractFunctionTreeRootRPCRequest {
    checkpoint_id: number;
    contract_id: number;
}

export interface QContractFunctionTreeRootFRPCRequest {
    checkpoint_id: bigint;
    contract_id: bigint;
}

export interface QContractFunctionTreeLeafHashRPCRequest {
    checkpoint_id: number;
    contract_id: number;
    function_id: number;
}

export interface QContractFunctionTreeLeafHashFRPCRequest {
    checkpoint_id: bigint;
    contract_id: bigint;
    function_id: bigint;
}

export interface QContractFunctionTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    contract_id: number;
    function_id: number;
}

export interface QContractFunctionTreeMerkleProofFRPCRequest {
    checkpoint_id: bigint;
    contract_id: bigint;
    function_id: bigint;
}

export interface QContractTreeRootRPCRequest {
    checkpoint_id: number;
}

export interface QContractTreeRootFRPCRequest {
    checkpoint_id: bigint;
}

export interface QContractTreeLeafHashRPCRequest {
    checkpoint_id: number;
    contract_id: number;
}

export interface QContractTreeLeafHashFRPCRequest {
    checkpoint_id: bigint;
    contract_id: bigint;
}

export interface QContractTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    contract_id: number;
}

export interface QContractTreeMerkleProofFRPCRequest {
    checkpoint_id: bigint;
    contract_id: bigint;
}

export interface QDepositTreeRootRPCRequest {
    checkpoint_id: number;
}

export interface QDepositTreeRootFRPCRequest {
    checkpoint_id: bigint;
}

export interface QDepositTreeLeafHashRPCRequest {
    checkpoint_id: number;
    deposit_id: number;
}

export interface QDepositTreeLeafHashFRPCRequest {
    checkpoint_id: bigint;
    deposit_id: bigint;
}

export interface QDepositTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    deposit_id: number;
}

export interface QDepositTreeMerkleProofFRPCRequest {
    checkpoint_id: bigint;
    deposit_id: bigint;
}

export interface QWithdrawalTreeRootRPCRequest {
    checkpoint_id: number;
}

export interface QWithdrawalTreeRootFRPCRequest {
    checkpoint_id: bigint;
}

export interface QWithdrawalTreeLeafHashRPCRequest {
    checkpoint_id: number;
    withdrawal_id: number;
}

export interface QWithdrawalTreeLeafHashFRPCRequest {
    checkpoint_id: bigint;
    withdrawal_id: bigint;
}

export interface QWithdrawalTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    withdrawal_id: number;
}

export interface QWithdrawalTreeMerkleProofFRPCRequest {
    checkpoint_id: bigint;
    withdrawal_id: bigint;
}

export interface QCheckpointTreeRootRPCRequest {
    checkpoint_id: number;
}

export interface QCheckpointTreeRootFRPCRequest {
    checkpoint_id: bigint;
}

export interface QCheckpointTreeLeafHashRPCRequest {
    checkpoint_id: number;
    leaf_checkpoint_id: number;
}

export interface QCheckpointTreeLeafHashFRPCRequest {
    checkpoint_id: bigint;
    leaf_checkpoint_id: bigint;
}

export interface QCheckpointTreeMerkleProofRPCRequest {
    checkpoint_id: number;
    leaf_checkpoint_id: number;
}

export interface QCheckpointTreeMerkleProofFRPCRequest {
    checkpoint_id: bigint;
    leaf_checkpoint_id: bigint;
}

/**
 * Coordinator Edge RPC Command namespace
 */
export enum CoordinatorEdgeRPCCommand {
    RegisterUser = "qed_register_user",
    GetUserId = "qed_get_user_id",
    DeployContract = "qed_deploy_contract",
    SubmitGUTA = "qed_submit_guta",
    GetLatestCheckpoint = "qed_get_latest_checkpoint",
    BuildBlock = "qed_build_block",
    GetCheckpointSyncInfo = "qed_get_checkpoint_sync_info",
    GetContractLeafData = "qed_get_contract_leaf_data",
    GetContractLeafDataF = "qed_get_contract_leaf_data_f",
    GetCheckpointLeafData = "qed_get_checkpoint_leaf_data",
    GetCheckpointLeafDataF = "qed_get_checkpoint_leaf_data_f",
    GetContractCodeDefinition = "qed_get_contract_code_definition",
    GetContractCodeDefinitionF = "qed_get_contract_code_definition_f",
    GetLatestL2BlockState = "qed_get_latest_l2_block_state",
    GetL2BlockState = "qed_get_l2_block_state",
    GetL2BlockStateF = "qed_get_l2_block_state_f",
    GetUserRegistrationTreeRoot = "qed_get_user_registration_tree_root",
    GetUserRegistrationTreeRootF = "qed_get_user_registration_tree_root_f",
    GetUserRegistrationTreeLeafHash = "qed_get_user_registration_tree_leaf_hash",
    GetUserRegistrationTreeLeafHashF = "qed_get_user_registration_tree_leaf_hash_f",
    GetUserRegistrationTreeMerkleProof = "qed_get_user_registration_tree_merkle_proof",
    GetUserRegistrationTreeMerkleProofF = "qed_get_user_registration_tree_merkle_proof_f",
    GetUserTreeRoot = "qed_get_user_tree_root",
    GetUserTreeRootF = "qed_get_user_tree_root_f",
    GetUserSubTreeMerkleProof = "qed_get_user_sub_tree_merkle_proof",
    GetUserTopTreeMerkleProof = "qed_get_user_top_tree_merkle_proof",
    GetUserTopTreeCapRoot = "qed_get_user_top_tree_cap_root",
    GetUserLatestTopTreeCapRoot = "qed_get_user_latest_top_tree_cap_root",
    GetContractFunctionTreeRoot = "qed_get_contract_function_tree_root",
    GetContractFunctionTreeRootF = "qed_get_contract_function_tree_root_f",
    GetContractFunctionTreeLeafHash = "qed_get_contract_function_tree_leaf_hash",
    GetContractFunctionTreeLeafHashF = "qed_get_contract_function_tree_leaf_hash_f",
    GetContractFunctionTreeMerkleProof = "qed_get_contract_function_tree_merkle_proof",
    GetContractFunctionTreeMerkleProofF = "qed_get_contract_function_tree_merkle_proof_f",
    GetContractTreeRoot = "qed_get_contract_tree_root",
    GetContractTreeRootF = "qed_get_contract_tree_root_f",
    GetContractTreeLeafHash = "qed_get_contract_tree_leaf_hash",
    GetContractTreeLeafHashF = "qed_get_contract_tree_leaf_hash_f",
    GetContractTreeMerkleProof = "qed_get_contract_tree_merkle_proof",
    GetContractTreeMerkleProofF = "qed_get_contract_tree_merkle_proof_f",
    GetDepositTreeRoot = "qed_get_deposit_tree_root",
    GetDepositTreeRootF = "qed_get_deposit_tree_root_f",
    GetDepositTreeLeafHash = "qed_get_deposit_tree_leaf_hash",
    GetDepositTreeLeafHashF = "qed_get_deposit_tree_leaf_hash_f",
    GetDepositTreeMerkleProof = "qed_get_deposit_tree_merkle_proof",
    GetDepositTreeMerkleProofF = "qed_get_deposit_tree_merkle_proof_f",
    GetWithdrawalTreeRoot = "qed_get_withdrawal_tree_root",
    GetWithdrawalTreeRootF = "qed_get_withdrawal_tree_root_f",
    GetWithdrawalTreeLeafHash = "qed_get_withdrawal_tree_leaf_hash",
    GetWithdrawalTreeLeafHashF = "qed_get_withdrawal_tree_leaf_hash_f",
    GetWithdrawalTreeMerkleProof = "qed_get_withdrawal_tree_merkle_proof",
    GetWithdrawalTreeMerkleProofF = "qed_get_withdrawal_tree_merkle_proof_f",
    GetLatestCheckpointTreeRoot = "qed_get_latest_checkpoint_tree_root",
    GetCheckpointTreeRoot = "qed_get_checkpoint_tree_root",
    GetCheckpointTreeRootF = "qed_get_checkpoint_tree_root_f",
    GetCheckpointTreeLeafHash = "qed_get_checkpoint_tree_leaf_hash",
    GetCheckpointTreeLeafHashF = "qed_get_checkpoint_tree_leaf_hash_f",
    GetCheckpointTreeMerkleProof = "qed_get_checkpoint_tree_merkle_proof",
    GetCheckpointTreeMerkleProofF = "qed_get_checkpoint_tree_merkle_proof_f",
    GetCheckpointGlobalStateRoots = "qed_get_checkpoint_global_state_roots",
    GetCheckpointSyncInfoCompact = "qed_get_checkpoint_sync_info_compact",
    LatestCheckpoint = "qed_latest_checkpoint",
    GetUserLeafData = "qed_get_user_leaf_data",
    GetUserTreeMerkleProof = "qed_get_user_tree_merkle_proof",
    GetUserTreeMerkleProofF = "qed_get_user_tree_merkle_proof_f",
}

/**
 * Interface for the Coordinator Edge RPC Provider
 */
export interface ICoordinatorEdgeRpcProvider {
    registerUser(pubKey: ZKPublicKeyInfo): Promise<string>;
    getUserId(qhash: QHashOut): Promise<number>;
    deployContract(contract: QBCDeployContract): Promise<string>;
    getLatestCheckpoint(): Promise<LatestCheckpointResponse>;
    buildBlock(): Promise<string>;
    getCheckpointSyncInfo(checkpointId: number): Promise<CheckpointSyncInfo>;
    getContractLeafData(contractId: number): Promise<QEDContractLeaf>;
    getContractLeafDataF(contractId: bigint): Promise<QEDContractLeaf>;
    getCheckpointLeafData(checkpointId: number): Promise<QEDCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: bigint): Promise<QEDCheckpointLeaf>;
    getContractCodeDefinition(contractId: number): Promise<ContractCodeDefinition>;
    getContractCodeDefinitionF(contractId: bigint): Promise<ContractCodeDefinition>;
    getLatestL2BlockState(): Promise<QEDL2BlockState>;
    getL2BlockState(checkpointId: number): Promise<QEDL2BlockState>;
    getL2BlockStateF(checkpointId: bigint): Promise<QEDL2BlockState>;
    getUserRegistrationTreeRoot(checkpointId: number): Promise<QHashOut>;
    getUserRegistrationTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getUserRegistrationTreeLeafHash(checkpointId: number, leafIndex: number): Promise<QHashOut>;
    getUserRegistrationTreeLeafHashF(checkpointId: bigint, leafIndex: bigint): Promise<QHashOut>;
    getUserRegistrationTreeMerkleProof(checkpointId: number, leafIndex: number): Promise<MerkleProofCore<QHashOut>>;
    getUserRegistrationTreeMerkleProofF(checkpointId: bigint, leafIndex: bigint): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeRoot(checkpointId: number): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getUserSubTreeMerkleProof(
        checkpointId: number,
        rootLevel: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeMerkleProof(
        checkpointId: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeCapRoot(checkpointId: number, capLevel: number, capIndex: number): Promise<QHashOut>;
    getUserLatestTopTreeCapRoot(capLevel: number, capIndex: number): Promise<QHashOut>;
    getContractFunctionTreeRoot(checkpointId: number, contractId: number): Promise<QHashOut>;
    getContractFunctionTreeRootF(checkpointId: bigint, contractId: bigint): Promise<QHashOut>;
    getContractFunctionTreeLeafHash(checkpointId: number, contractId: number, functionId: number): Promise<QHashOut>;
    getContractFunctionTreeLeafHashF(checkpointId: bigint, contractId: bigint, functionId: bigint): Promise<QHashOut>;
    getContractFunctionTreeMerkleProof(
        checkpointId: number,
        contractId: number,
        functionId: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getContractFunctionTreeMerkleProofF(
        checkpointId: bigint,
        contractId: bigint,
        functionId: bigint
    ): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeRoot(checkpointId: number): Promise<QHashOut>;
    getContractTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getContractTreeLeafHash(checkpointId: number, contractId: number): Promise<QHashOut>;
    getContractTreeLeafHashF(checkpointId: bigint, contractId: bigint): Promise<QHashOut>;
    getContractTreeMerkleProof(checkpointId: number, contractId: number): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeMerkleProofF(checkpointId: bigint, contractId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeRoot(checkpointId: number): Promise<QHashOut>;
    getDepositTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getDepositTreeLeafHash(checkpointId: number, depositId: number): Promise<QHashOut>;
    getDepositTreeLeafHashF(checkpointId: bigint, depositId: bigint): Promise<QHashOut>;
    getDepositTreeMerkleProof(checkpointId: number, depositId: number): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeMerkleProofF(checkpointId: bigint, depositId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeRoot(checkpointId: number): Promise<QHashOut>;
    getWithdrawalTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getWithdrawalTreeLeafHash(checkpointId: number, withdrawalId: number): Promise<QHashOut>;
    getWithdrawalTreeLeafHashF(checkpointId: bigint, withdrawalId: bigint): Promise<QHashOut>;
    getWithdrawalTreeMerkleProof(checkpointId: number, withdrawalId: number): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeMerkleProofF(checkpointId: bigint, withdrawalId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: number): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getCheckpointTreeLeafHash(checkpointId: number, leafCheckpointId: number): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: bigint, leafCheckpointId: bigint): Promise<QHashOut>;
    getCheckpointTreeMerkleProof(checkpointId: number, leafCheckpointId: number): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: bigint, leafCheckpointId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointGlobalStateRoots(checkpointId: number): Promise<QEDCheckpointGlobalStateRoots>;
    getCheckpointSyncInfoCompact(checkpointId: number): Promise<QEDCheckpointSyncInfoCompact>;
    latestCheckpoint(): Promise<number>;
    getUserLeafData(checkpointId: number, userId: number): Promise<QEDUserLeaf>;
    getUserTreeMerkleProof(checkpointId: number, userId: number): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: bigint, userId: bigint): Promise<MerkleProofCore<QHashOut>>;
}
