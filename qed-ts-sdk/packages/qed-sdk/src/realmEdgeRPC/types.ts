import { Hash256, HexString, SCFelt } from "../rpc/baseTypes";
import { ProofWithPublicInputs } from "../rpc/plonkTypes";

// QHash type from Rust
export interface QHashOut {
    elements: bigint[];
}

// Merkle proof core structure
export interface MerkleProofCore<T> {
    root: T;
    value: T;
    index: bigint;
    siblings: T[];
}

// Checkpoint leaf data
export interface QEDCheckpointLeaf {
    checkpoint_id: bigint;
    next_add_withdrawal_id: bigint;
    next_process_withdrawal_id: bigint;
    next_deposit_id: bigint;
    total_deposits_claimed_epoch: bigint;
    next_user_id: bigint;
    end_balance: bigint;
}

// L2 block state
export interface QEDL2BlockState {
    checkpoint_id: bigint;
    next_add_withdrawal_id: bigint;
    next_process_withdrawal_id: bigint;
    next_deposit_id: bigint;
    total_deposits_claimed_epoch: bigint;
    next_user_id: bigint;
    end_balance: bigint;
    next_contract_id: bigint;
}

// Checkpoint global state roots
export interface QEDCheckpointGlobalStateRoots {
    user_tree_root: QHashOut;
    checkpoint_tree_root: QHashOut;
    withdrawal_tree_root: QHashOut;
    deposit_tree_root: QHashOut;
}

// User leaf data
export interface QEDUserLeaf {
    user_id: bigint;
    nonce: bigint;
    last_checkpoint_id: bigint;
    user_state_tree_root: QHashOut;
    user_contract_tree_root: QHashOut;
    user_pk_hash: QHashOut;
}

// Contract state updates
export interface QEDContractStateUpdateHistory {
    contract_id: bigint;
    updates: any[]; // This is a placeholder, replace with actual type if needed
}

// User EndCap core input
export interface SubmitUserEndCapNonProofCoreInput {
    checkpoint_id: bigint;
    stats: any; // GUTAStats
    state_transition: any; // UPSEndCapResultCompact
    new_user_leaf: QEDUserLeaf;
}

// Full NonProof input
export interface SubmitUserEndCapNonProofInput {
    core: SubmitUserEndCapNonProofCoreInput;
    contract_state_updates: QEDContractStateUpdateHistory[];
}

// RPC Method commands enum
export enum RealmEdgeRPCCommand {
    CheckUserIdInRealm = "qed_check_user_id_in_realm",
    SubmitUserEndCap = "qed_submit_user_end_cap",
    GetCheckpointLeafData = "qed_get_checkpoint_leaf_data",
    GetCheckpointLeafDataF = "qed_get_checkpoint_leaf_data_f",
    GetLatestL2BlockState = "qed_get_latest_l2_block_state",
    GetL2BlockState = "qed_get_l2_block_state",
    GetL2BlockStateF = "qed_get_l2_block_state_f",
    GetUserRegistrationTreeRoot = "qed_get_user_registration_tree_root",
    GetLatestCheckpointTreeRoot = "qed_get_latest_checkpoint_tree_root",
    GetCheckpointTreeRoot = "qed_get_checkpoint_tree_root",
    GetCheckpointTreeRootF = "qed_get_checkpoint_tree_root_f",
    GetCheckpointTreeLeafHash = "qed_get_checkpoint_tree_leaf_hash",
    GetCheckpointTreeLeafHashF = "qed_get_checkpoint_tree_leaf_hash_f",
    GetCheckpointTreeMerkleProof = "qed_get_checkpoint_tree_merkle_proof",
    GetCheckpointTreeMerkleProofF = "qed_get_checkpoint_tree_merkle_proof_f",
    GetCheckpointGlobalStateRoots = "qed_get_checkpoint_global_state_roots",
    GetUserLeafData = "qed_get_user_leaf_data",
    GetUserLeafDataF = "qed_get_user_leaf_data_f",
    GetUserContractStateTreeRoot = "qed_get_user_contract_state_tree_root",
    GetUserContractStateTreeRootF = "qed_get_user_contract_state_tree_root_f",
    GetUserContractStateTreeLeafHash = "qed_get_user_contract_state_tree_leaf_hash",
    GetUserContractStateTreeLeafHashF = "qed_get_user_contract_state_tree_leaf_hash_f",
    GetUserContractStateTreeMerkleProof = "qed_get_user_contract_state_tree_merkle_proof",
    GetUserContractStateTreeMerkleProofF = "qed_get_user_contract_state_tree_merkle_proof_f",
    GetUserContractTreeRoot = "qed_get_user_contract_tree_root",
    GetUserContractTreeRootF = "qed_get_user_contract_tree_root_f",
    GetUserContractTreeLeafHash = "qed_get_user_contract_tree_leaf_hash",
    GetUserContractTreeLeafHashF = "qed_get_user_contract_tree_leaf_hash_f",
    GetUserContractTreeMerkleProof = "qed_get_user_contract_tree_merkle_proof",
    GetUserContractTreeMerkleProofF = "qed_get_user_contract_tree_merkle_proof_f",
    GetUserTreeRoot = "qed_get_user_tree_root",
    GetUserTreeRootF = "qed_get_user_tree_root_f",
    GetUserTreeLeafHash = "qed_get_user_tree_leaf_hash",
    GetUserTreeLeafHashF = "qed_get_user_tree_leaf_hash_f",
    GetUserBottomTreeMerkleProof = "qed_get_user_bottom_tree_merkle_proof",
    GetUserBottomTreeMerkleProofF = "qed_get_user_bottom_tree_merkle_proof_f",
    GetUserSubTreeMerkleProof = "qed_get_user_sub_tree_merkle_proof",
    GetUserSubTreeMerkleProofF = "qed_get_user_sub_tree_merkle_proof_f",
    GetUserTreeMerkleProof = "qed_get_user_tree_merkle_proof",
    GetUserTreeMerkleProofF = "qed_get_user_tree_merkle_proof_f",
}

// Interface for the Realm Edge RPC client
export interface IRealmEdgeRpcProvider {
    // Check user ID
    checkUserIdInRealm(userId: bigint | number): Promise<boolean>;

    // Submit user end cap
    submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string>;

    // Get checkpoint leaf data
    getCheckpointLeafData(checkpointId: bigint | number): Promise<QEDCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: bigint): Promise<QEDCheckpointLeaf>;

    // Get L2 block state
    getLatestL2BlockState(): Promise<QEDL2BlockState>;
    getL2BlockState(checkpointId: bigint | number): Promise<QEDL2BlockState>;
    getL2BlockStateF(checkpointId: bigint): Promise<QEDL2BlockState>;

    // Get user registration tree root
    getUserRegistrationTreeRoot(checkpointId: bigint | number): Promise<QHashOut>;

    // Get checkpoint tree roots
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: bigint | number): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: bigint): Promise<QHashOut>;

    // Get checkpoint tree leaf hash
    getCheckpointTreeLeafHash(checkpointId: bigint | number, leafCheckpointId: bigint | number): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: bigint, leafCheckpointId: bigint): Promise<QHashOut>;

    // Get checkpoint tree merkle proof
    getCheckpointTreeMerkleProof(
        checkpointId: bigint | number,
        leafCheckpointId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: bigint, leafCheckpointId: bigint): Promise<MerkleProofCore<QHashOut>>;

    // Get checkpoint global state roots
    getCheckpointGlobalStateRoots(checkpointId: bigint | number): Promise<QEDCheckpointGlobalStateRoots>;

    // Get user leaf data
    getUserLeafData(checkpointId: bigint | number, userId: bigint | number): Promise<QEDUserLeaf>;
    getUserLeafDataF(checkpointId: bigint, userId: bigint): Promise<QEDUserLeaf>;

    // Get user contract state tree root
    getUserContractStateTreeRoot(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<QHashOut>;
    getUserContractStateTreeRootF(checkpointId: bigint, userId: bigint, contractId: bigint): Promise<QHashOut>;

    // Get user contract state tree leaf hash
    getUserContractStateTreeLeafHash(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number,
        height: number,
        leafId: bigint | number
    ): Promise<QHashOut>;
    getUserContractStateTreeLeafHashF(
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint,
        height: number,
        leafId: bigint
    ): Promise<QHashOut>;

    // Get user contract state tree merkle proof
    getUserContractStateTreeMerkleProof(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number,
        height: number,
        leafId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserContractStateTreeMerkleProofF(
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint,
        height: number,
        leafId: bigint
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user contract tree root
    getUserContractTreeRoot(checkpointId: bigint | number, userId: bigint | number): Promise<QHashOut>;
    getUserContractTreeRootF(checkpointId: bigint, userId: bigint): Promise<QHashOut>;

    // Get user contract tree leaf hash
    getUserContractTreeLeafHash(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<QHashOut>;
    getUserContractTreeLeafHashF(checkpointId: bigint, userId: bigint, contractId: bigint): Promise<QHashOut>;

    // Get user contract tree merkle proof
    getUserContractTreeMerkleProof(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeMerkleProofF(
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user tree root
    getUserTreeRoot(checkpointId: bigint | number): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: bigint): Promise<QHashOut>;

    // Get user tree leaf hash
    getUserTreeLeafHash(checkpointId: bigint | number, userId: bigint | number): Promise<QHashOut>;
    getUserTreeLeafHashF(checkpointId: bigint, userId: bigint): Promise<QHashOut>;

    // Get user bottom tree merkle proof
    getUserBottomTreeMerkleProof(
        rootLevel: number,
        checkpointId: bigint | number,
        userId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserBottomTreeMerkleProofF(
        rootLevel: number,
        checkpointId: bigint,
        userId: bigint
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user sub tree merkle proof
    getUserSubTreeMerkleProof(
        checkpointId: bigint | number,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint | number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProofF(
        checkpointId: bigint,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user tree merkle proof
    getUserTreeMerkleProof(checkpointId: bigint | number, userId: bigint | number): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: bigint, userId: bigint): Promise<MerkleProofCore<QHashOut>>;
}
