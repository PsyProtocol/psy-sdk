import { QHashOut, MerkleProofCore, Felt } from "../core";
import { ProofWithPublicInputs, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf, PsyBlockState, PsyUserLeaf, SubmitUserEndCapNonProofInput } from "../types";
export declare enum RealmEdgeRPCCommand {
    CheckUserIdInRealm = "psy_check_user_id_in_realm",
    SubmitUserEndCap = "psy_submit_user_end_cap",
    GetCheckpointLeafData = "psy_get_checkpoint_leaf_data",
    GetCheckpointLeafDataF = "psy_get_checkpoint_leaf_data_f",
    GetLatestBlockState = "psy_get_latest_l2_block_state",
    GetBlockState = "psy_get_l2_block_state",
    GetBlockStateF = "psy_get_l2_block_state_f",
    GetUserRegistrationTreeRoot = "psy_get_user_registration_tree_root",
    GetLatestCheckpointTreeRoot = "psy_get_latest_checkpoint_tree_root",
    GetCheckpointTreeRoot = "psy_get_checkpoint_tree_root",
    GetCheckpointTreeRootF = "psy_get_checkpoint_tree_root_f",
    GetCheckpointTreeLeafHash = "psy_get_checkpoint_tree_leaf_hash",
    GetCheckpointTreeLeafHashF = "psy_get_checkpoint_tree_leaf_hash_f",
    GetCheckpointTreeMerkleProof = "psy_get_checkpoint_tree_merkle_proof",
    GetCheckpointTreeMerkleProofF = "psy_get_checkpoint_tree_merkle_proof_f",
    GetCheckpointGlobalStateRoots = "psy_get_checkpoint_global_state_roots",
    GetUserLeafData = "psy_get_user_leaf_data",
    GetUserLeafDataF = "psy_get_user_leaf_data_f",
    GetUserContractStateTreeRoot = "psy_get_user_contract_state_tree_root",
    GetUserContractStateTreeRootF = "psy_get_user_contract_state_tree_root_f",
    GetUserContractStateTreeLeafHash = "psy_get_user_contract_state_tree_leaf_hash",
    GetUserContractStateTreeLeafHashF = "psy_get_user_contract_state_tree_leaf_hash_f",
    GetUserContractStateTreeMerkleProof = "psy_get_user_contract_state_tree_merkle_proof",
    GetUserContractStateTreeMerkleProofF = "psy_get_user_contract_state_tree_merkle_proof_f",
    GetUserContractTreeRoot = "psy_get_user_contract_tree_root",
    GetUserContractTreeRootF = "psy_get_user_contract_tree_root_f",
    GetUserContractTreeLeafHash = "psy_get_user_contract_tree_leaf_hash",
    GetUserContractTreeLeafHashF = "psy_get_user_contract_tree_leaf_hash_f",
    GetUserContractTreeMerkleProof = "psy_get_user_contract_tree_merkle_proof",
    GetUserContractTreeMerkleProofF = "psy_get_user_contract_tree_merkle_proof_f",
    GetUserTreeRoot = "psy_get_user_tree_root",
    GetUserTreeRootF = "psy_get_user_tree_root_f",
    GetUserTreeLeafHash = "psy_get_user_tree_leaf_hash",
    GetUserTreeLeafHashF = "psy_get_user_tree_leaf_hash_f",
    GetUserBottomTreeMerkleProof = "psy_get_user_bottom_tree_merkle_proof",
    GetUserBottomTreeMerkleProofF = "psy_get_user_bottom_tree_merkle_proof_f",
    GetUserSubTreeMerkleProof = "psy_get_user_sub_tree_merkle_proof",
    GetUserSubTreeMerkleProofF = "psy_get_user_sub_tree_merkle_proof_f",
    GetUserTreeMerkleProof = "psy_get_user_tree_merkle_proof",
    GetUserTreeMerkleProofF = "psy_get_user_tree_merkle_proof_f"
}
export interface IRealmEdgeRpcProvider {
    setUserId(userId: Felt): void;
    getRpcProviderByUserId(userId: Felt): IRealmEdgeRpcProvider;
    checkUserIdInRealm(userId: Felt): Promise<boolean>;
    submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string>;
    getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getLatestBlockState(): Promise<PsyBlockState>;
    getBlockState(checkpointId: Felt): Promise<PsyBlockState>;
    getBlockStateF(checkpointId: Felt): Promise<PsyBlockState>;
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots>;
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserContractStateTreeRoot(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractStateTreeRootF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractStateTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<QHashOut>;
    getUserContractStateTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<QHashOut>;
    getSlotValue(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, slot: Felt): Promise<Felt>;
    getSlotValues(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, slots: Felt[]): Promise<Felt[]>;
    getUserContractStateTreeMerkleProof(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractStateTreeMerkleProofF(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeRoot(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserContractTreeRootF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserContractTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractTreeMerkleProof(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeMerkleProofF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeLeafHash(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserTreeLeafHashF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserBottomTreeMerkleProof(rootLevel: number, checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserBottomTreeMerkleProofF(rootLevel: number, checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProof(checkpointId: Felt, rootLevel: number, leafLevel: number, leafIndex: bigint | number): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProofF(checkpointId: Felt, rootLevel: number, leafLevel: number, leafIndex: bigint): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
}
//# sourceMappingURL=types.d.ts.map