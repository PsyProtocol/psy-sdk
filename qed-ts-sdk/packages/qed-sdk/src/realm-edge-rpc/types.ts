import { QHashOut, MerkleProofCore, Felt } from "../core";
import {
    ProofWithPublicInputs,
    QEDCheckpointGlobalStateRoots,
    QEDCheckpointLeaf,
    QEDL2BlockState,
    QEDUserLeaf,
    SubmitUserEndCapNonProofInput,
} from "../types";

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
    // Set UserId
    // setUserId(userId: Felt): void;
    getRpcProviderByUserId(userId: Felt): IRealmEdgeRpcProvider;

    // Check user ID
    checkUserIdInRealm(userId: Felt): Promise<boolean>;

    // Submit user end cap
    submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string>;

    // Get checkpoint leaf data
    getCheckpointLeafData(checkpointId: Felt): Promise<QEDCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: Felt): Promise<QEDCheckpointLeaf>;

    // Get L2 block state
    getLatestL2BlockState(): Promise<QEDL2BlockState>;
    getL2BlockState(checkpointId: Felt): Promise<QEDL2BlockState>;
    getL2BlockStateF(checkpointId: Felt): Promise<QEDL2BlockState>;

    // Get user registration tree root
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut>;

    // Get checkpoint tree roots
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut>;

    // Get checkpoint tree leaf hash
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;

    // Get checkpoint tree merkle proof
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;

    // Get checkpoint global state roots
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<QEDCheckpointGlobalStateRoots>;

    // Get user leaf data
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf>;
    getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf>;

    // Get user contract state tree root
    getUserContractStateTreeRoot(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractStateTreeRootF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;

    // Get user contract state tree leaf hash
    getUserContractStateTreeLeafHash(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<QHashOut>;
    getUserContractStateTreeLeafHashF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<QHashOut>;

    // Get user contract state tree merkle proof
    getUserContractStateTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserContractStateTreeMerkleProofF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user contract tree root
    getUserContractTreeRoot(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserContractTreeRootF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;

    // Get user contract tree leaf hash
    getUserContractTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;

    // Get user contract tree merkle proof
    getUserContractTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeMerkleProofF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user tree root
    getUserTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: Felt): Promise<QHashOut>;

    // Get user tree leaf hash
    getUserTreeLeafHash(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserTreeLeafHashF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;

    // Get user bottom tree merkle proof
    getUserBottomTreeMerkleProof(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserBottomTreeMerkleProofF(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user sub tree merkle proof
    getUserSubTreeMerkleProof(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint | number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProofF(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint
    ): Promise<MerkleProofCore<QHashOut>>;

    // Get user tree merkle proof
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
}
